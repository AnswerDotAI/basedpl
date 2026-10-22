//! The binder: the right-to-left reduction of a run's items by their categories, and the rewriting of dot access.

use super::*;
use super::tree::{Facts, Recorder, Reduction};

impl Session {
    fn has_members(nodes: &[Node]) -> bool {
        nodes.iter().any(|n| matches!(n.kind, NodeKind::Subscript(_) | NodeKind::Joiner(_))) || nodes.windows(2).any(|w| {
            matches!(w[0].kind, NodeKind::Operator(OperatorKind::Product))
                && matches!(w[1].kind, NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. })
        })
    }

    // After a value, `.name` is `'name'⊃value` and `.[I]` is bracket indexing, which pairs index arrays by broadcasting. `.(expr)` evaluates `expr` with the value's keys as names. Between functions the dot stays inner product. A subscript then selects from the item before it, so `T.a₁` is `1⌷'a'⊃T`.
    pub(super) fn members<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> { self.members_with(nodes, &mut None) }
    /// `members`, logging in `log` what the rewrite read about what names hold (`tree::Fact`).
    pub(super) fn members_with<'a>(&self, nodes: &'a [Node], log: &mut Facts) -> Cow<'a, [Node]> {
        if !Self::has_members(nodes) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        // Whether the last node in `out` is the right operand of a `.` after a function.
        let (mut i, mut on_function) = (0, false);
        while i < nodes.len() {
            if let (NodeKind::Operator(OperatorKind::Product), Some(next)) = (&nodes[i].kind, nodes.get(i + 1)) {
                let root = out.last().filter(|root| !on_function && match &root.kind {
                    NodeKind::Name(_) | NodeKind::Group(_) => self.holds_array(root, log),
                    NodeKind::Literal(_) | NodeKind::ArrayLiteral { .. } | NodeKind::Scope(..) => true, _ => false
                });
                // `.name` reads a field and `.[I]` selects, after an array or a function. After a function, a name that starts a dot path to a function is an inner product's operand instead, as in `+.ops.mul`.
                let read = match &next.kind {
                    NodeKind::Name(name) if root.is_some() || !self.path_function(&nodes[i + 1..], log) => {
                        Some((Node { kind: NodeKind::Literal(crate::keyed::text(name.text())), span: next.span.clone() }, Primitive::Mix))
                    }
                    NodeKind::ArrayLiteral { form: ListForm::Items | ListForm::Cells, .. } => Some((next.clone(), Primitive::Bracket)),
                    _ => None,
                };
                if let Some(root) = root {
                    let kind = match (read, &next.kind) {
                        (Some((key, function)), _) => Some(NodeKind::Group(vec![key, Node { kind: NodeKind::Function(Builtin::Primitive(function)), span: nodes[i].span.clone() }, root.clone()])),
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
                out.push(Node { kind: NodeKind::Group(vec![index, Node { kind: NodeKind::Function(Builtin::Primitive(Primitive::Index)), span: nodes[i].span.clone() }, item]), span
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
                    || !self.holds_array(&left, log)
                    || !self.holds_array(&out[i + 1], log) { i += 1; continue; }
                let joined = joined_term(left, joiner, &out[i].span, out[i + 1].clone());
                out.splice(start..i + 2, [joined]);
                i = start + 1;
            }
        }
        Cow::Owned(out)
    }

    /// Whether a run ends in a dyadic operator, which then takes the next run as its right operand.
    pub(super) fn ends_in_dyadic_operator(&self, nodes: &[Node], log: &mut Facts) -> bool { self.members_with(nodes, log).last().is_some_and(|n| self.static_category(n, log).valence() == Some(true)) }

    /// Whether a run starts with an operator, which then takes the whole run before it as its left operand.
    pub(super) fn starts_with_operator(&self, nodes: &[Node], log: &mut Facts) -> bool { self.members_with(nodes, log).first().is_some_and(|n| self.static_category(n, log).valence().is_some()) }

    /// Join each run that starts with an operator to the node before it, as the run's first token: `≢⊢ ⌸x` reads as `(≢⊢)⌸x`. A run before it joins as a group. A joined run that still starts with an operator joins again: `+ / ¨x` reads as `+/¨x`. No join reaches across `←`. `log` keeps what each test read about what names hold (`tree::Fact`).
    fn joined<'a>(&self, nodes: &'a [Node], log: &mut Facts) -> Cow<'a, [Node]> {
        let starts = |node: &Node, log: &mut Facts| matches!(&node.kind, NodeKind::Run(inner) if self.starts_with_operator(inner, log));
        // Every run but the first is tested, so that `log` keeps what each test read.
        if !nodes.iter().skip(1).fold(false, |any, node| starts(node, log) || any) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        let mut unlogged = None;
        for node in nodes {
            let mut node = node.clone();
            // The test above read each node as written. A run that takes in the node before it is a new run, whose test logs what it reads.
            let mut merged = false;
            while out.last().is_some_and(|before| !matches!(before.kind, NodeKind::Assign)) && starts(&node, if merged { &mut *log } else { &mut unlogged }) {
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
    /// Whether the dot path that starts `nodes`, such as `mul` or `ops.mul`, names a function or operator, read without evaluating anything (`path_category`).
    fn path_function(&self, nodes: &[Node], log: &mut Facts) -> bool {
        let Some(NodeKind::Name(name)) = nodes.first().map(|n| &n.kind) else { return false };
        let keys: Vec<Arc<str>> = nodes[1..].chunks_exact(2).map_while(|pair| match pair {
            [Node { kind: NodeKind::Operator(OperatorKind::Product), .. }, Node { kind: NodeKind::Name(key), .. }] => Some(key.text().into()),
            _ => None,
        }).collect();
        self.path_category(*name, &keys, log) != Category::Value
    }
}

// Only the binder has unfinished trains and bound left arguments. Names and groups contain completed values. `Callable` is an array that holds a function, which binds by the item to its right (`Category::before`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category { NoResult, Value, Function, Operator, DyadicOperator, Left, Train, Absent, Callable }
#[derive(Clone, Debug)]
pub(super) enum Term {
    Binding(Binding),
    /// Functions and arrays side by side before a function with no argument.
    Train(Vec<Tine>),
    Left(Value, Function),
    /// Arrays side by side, one item each. They become a vector when used.
    Strand(Vec<Value>),
    /// A group's train, held as its tines until something needs its function. A compiled call of the group calls the tines in place (`tree.rs`'s `Op::CallTines`).
    Tines(Vec<Tine>),
    /// A keyed bracket item's entries, which only its literal reads, in a compiled statement.
    Entries(Vec<crate::keyed::Entry>),
}
#[derive(Clone, Debug)]
pub(super) enum Tine { Array(Value), Function(Function) }
/// The last entity of a run, or the call it ends in when that call is in tail position.
enum Reduced { Entity(Entity), Tail(Application) }
impl Reduced {
    /// The step that a run gives: its call in tail position, or its last entity's value.
    fn step(self, source: &Arc<Source>) -> Result<Step, Error> { Ok(match self { Self::Tail(call) => Step::Tail(call), Self::Entity(entity) => Step::Done(entity.bound(source)?) }) }
}
#[derive(Clone, Debug)]
pub(super) struct Entity {
    pub(super) term: Term,
    /// The `u32` offsets of the entity's text in the source of the binder's nodes. They keep `Entity` within 64 bytes, which the assertion below checks. A `Span` is rebuilt from them only where one is needed.
    pub(super) range: std::ops::Range<u32>,
    pub(super) shy: bool,
    /// The entity's own category, from its term (`Category::bound`).
    category: Category,
    /// The category it binds with on the binder's stack: its `category`, with a function array resolved against the entity below it (`Binder::push`).
    role: Category,
    pub(super) expression: Option<Box<Expression>>,
}
/// The binder's state where a recording's check failed (`tree.rs`): its stack, its pending stack and its cursor, with the entity it had just resolved, whether it had just pushed a call's result, and the `←` of an assignment it was making.
pub(super) struct Resume { pub(super) stack: Vec<Entity>, pub(super) pending: Vec<Entity>, pub(super) cursor: usize, pub(super) next: Option<Entity>, pub(super) after_call: bool, pub(super) assignment: Option<usize> }
const _: () = assert!(std::mem::size_of::<Entity>() <= 64);

/// `range` as `u32` offsets, clamped for a source longer than 4 GiB.
pub(super) fn narrow(range: &std::ops::Range<usize>) -> std::ops::Range<u32> { let at = |i: usize| u32::try_from(i).unwrap_or(u32::MAX); at(range.start)..at(range.end) }
/// The span of `range` in `source`.
pub(super) fn span_of(source: &Arc<Source>, range: &std::ops::Range<u32>) -> Span { Span { source: source.clone(), range: range.start as usize..range.end as usize } }

impl Entity {
    /// An entity with `term`, not yet on the binder's stack.
    // Out of line, `new` built each entity and then copied it out, about 265 instructions for each call of `{x←⍵+1 ⋄ x}`.
    #[inline(always)]
    pub(super) fn new(term: Term, range: std::ops::Range<u32>, shy: bool, expression: Option<Box<Expression>>) -> Self {
        let category = match term {
            Term::Binding(ref value) => Category::bound(value), Term::Strand(_) | Term::Entries(_) => Category::Value, Term::Train(_) => Category::Train,
            Term::Left(..) => Category::Left, Term::Tines(_) => Category::Function
        };
        Self { term, range, shy, category, role: category, expression }
    }
    /// An entity that holds `value`, with no captured expression.
    #[inline(always)]
    pub(super) fn of(value: Binding, range: std::ops::Range<u32>, shy: bool) -> Self { Self::new(Term::Binding(value), range, shy, None) }
    /// The entity's own category.
    #[inline(always)]
    pub(super) fn category(&self) -> Category { self.category }
    /// The entity's value: its binding, or what its unfinished term builds (`Entity::built`).
    #[inline(always)]
    pub(super) fn value(self, source: &Arc<Source>) -> Result<Binding, Error> {
        match self.term { Term::Binding(value) => Ok(value), term => (Self { term, ..self }).built(source) }
    }
    /// What an unfinished term builds: a train's function, a bound left argument's function or a strand's vector.
    // Out of line, so that the conversions that every compiled operand goes through stay small enough to inline.
    #[inline(never)]
    fn built(self, source: &Arc<Source>) -> Result<Binding, Error> {
        let span = || span_of(source, &self.range);
        Ok(match self.term {
            Term::Binding(value) => value, Term::Train(tines) | Term::Tines(tines) => Binding::Function(self::Function::train(tines, &span())?),
            Term::Left(a, f) => Binding::Function(before(a, f, &span())?),
            Term::Strand(items) => Binding::Value(Value::new(vec![items.len()], items).map_err(|k| Error::new(k, "invalid strand").at(&span()))?),
            Term::Entries(_) => unreachable!("a keyed item's entries go only into its literal"),
        })
    }
    #[inline(always)]
    pub(super) fn function(self, source: &Arc<Source>) -> Result<Function, Error> {
        let range = self.range.clone();
        match self.value(source)? {
            Binding::Function(f) => Ok(f),
            value => Function::from_value(value, &span_of(source, &range))
        }
    }
    #[inline(always)]
    pub(super) fn array(self, source: &Arc<Source>) -> Result<Value, Error> { match self.value(source)? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
    /// The entity's value, with whether it displays and the expression it captured.
    fn bound(mut self, source: &Arc<Source>) -> Result<Bound, Error> {
        let (shy, expression) = (self.shy, self.expression.take());
        Ok(Bound { value: self.value(source)?, shy, expression })
    }
}

/// `left` and `right` side by side as a strand. A strand adds its items, and anything else adds itself as one array. Gives the items, how many of them came from `left`, and the strand's text range, which is `left`'s.
#[inline]
pub(super) fn strand(left: Entity, right: Entity, source: &Arc<Source>) -> Result<(Vec<Value>, usize, std::ops::Range<u32>), Error> {
    let range = left.range.clone();
    let mut joined = match left.term { Term::Strand(items) => items, term => { let mut items = Vec::with_capacity(2); items.push(Entity { term, ..left }.array(source)?); items } };
    let from_left = joined.len();
    match right.term { Term::Strand(items) => joined.extend(items), term => joined.push(Entity { term, ..right }.array(source)?) }
    Ok((joined, from_left, range))
}

/// `entity` as an argument: its array, which stays borrowed where a borrowed entity holds one, as a recording's constant does (`tree.rs`).
#[inline(always)]
pub(super) fn array_of<'p>(entity: Cow<'p, Entity>, source: &Arc<Source>) -> Result<Cow<'p, Value>, Error> {
    match entity {
        Cow::Borrowed(Entity { term: Term::Binding(Binding::Value(v)), .. }) => Ok(Cow::Borrowed(v)),
        entity => entity.into_owned().array(source).map(Cow::Owned),
    }
}

/// `entity` as a called function, which stays borrowed where a borrowed entity holds one.
#[inline(always)]
pub(super) fn function_of<'p>(entity: Cow<'p, Entity>, source: &Arc<Source>) -> Result<Cow<'p, Function>, Error> {
    match entity {
        Cow::Borrowed(Entity { term: Term::Binding(Binding::Function(f)), .. }) => Ok(Cow::Borrowed(f)),
        entity => entity.into_owned().function(source).map(Cow::Owned),
    }
}

/// `left` bound as `right`'s left argument: `left` as an argument, then `right` as a function. The pair takes `right`'s text range, because a call belongs to its function's text.
#[inline]
pub(super) fn attached<'p>(left: Cow<'p, Entity>, right: Cow<'p, Entity>, source: &Arc<Source>) -> Result<(Cow<'p, Value>, Cow<'p, Function>, std::ops::Range<u32>), Error> {
    let range = right.range.clone();
    let x = array_of(left, source)?;
    Ok((x, function_of(right, source)?, range))
}

/// A call's left argument and function, from the entity before its right argument: a bound left argument's parts, or no left argument and the entity as a function.
#[inline]
pub(super) fn split_call<'p>(left: Cow<'p, Entity>, source: &Arc<Source>) -> Result<(Option<Cow<'p, Value>>, Cow<'p, Function>), Error> {
    Ok(match left {
        Cow::Owned(Entity { term: Term::Left(x, f), .. }) => (Some(Cow::Owned(x)), Cow::Owned(f)),
        Cow::Borrowed(Entity { term: Term::Left(x, f), .. }) => (Some(Cow::Borrowed(x)), Cow::Borrowed(f)),
        left => (None, function_of(left, source)?),
    })
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
            (Superscript::Unit, Operand::Function(f)) => return Ok(Derived::Term(Binding::Value(crate::Value::Function(f).enclose().described("invalid scalar").at(span)?))),
            (Superscript::Unit, _) => return Err(Error::domain("ᵘ makes a scalar that holds a function. Enclose a subject with ⊂").at(span)),
            (Superscript::Power(n), Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Math(crate::number::Math::Power)), Some(a), crate::Value::integer(n))),
            (Superscript::Transpose, Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Transpose), None, a)),
            (Superscript::Floor, Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Math(crate::number::Math::Floor)), None, a)),
            (_, operand) => return Ok(Derived::Term(operator.derive(operand, span)?)),
        }
    }
    Ok(Derived::Term(operator.derive(operand, span)?))
}

/// The dyadic operator `operator` with its right operand `operand` bound (`Rule::BindRight`).
pub(super) fn bound_right(operator: Binding, operand: Binding) -> self::OperatorNode {
    let Binding::Operator(operator) = operator else { unreachable!("a bind's left item is a dyadic operator") };
    self::OperatorNode::Bound(Arc::new(operator), Operand::from_value(operand))
}

/// The tines of `left` and then of `right`, which join in a train (`Rule::Train`). The tines go into a train's own list where either side brings one, and otherwise into a new list with room for `TINES`, so a statement that builds a train at each call allocates once for it.
#[inline]
pub(super) fn train(left: Cow<'_, Entity>, right: Cow<'_, Entity>, source: &Arc<Source>) -> Result<Vec<Tine>, Error> {
    Ok(match (left, right) {
        (Cow::Owned(Entity { term: Term::Train(mut tines), .. }), right) => { push_tines(right, &mut tines, source)?; tines }
        (left, Cow::Owned(Entity { term: Term::Train(mut tines), .. })) => {
            let n = tines.len();
            push_tines(left, &mut tines, source)?;
            let pushed = tines.len() - n;
            tines.rotate_right(pushed);
            tines
        }
        (left, right) => {
            let mut tines = Vec::with_capacity(TINES);
            push_tines(left, &mut tines, source)?;
            push_tines(right, &mut tines, source)?;
            tines
        }
    })
}

/// The tines in a train's new list that a statement such as `(g ⍵-1)+g ⍵-2` fills without growing it.
const TINES: usize = 4;

/// Appends `entity`'s parts to `tines` as tines of a train: a train's tines, a bound left argument's array and function, or the entity as one array or function. Where `entity` is borrowed, as a recording's constant is (`tree.rs`), each tine copies only its own array or function.
#[inline]
fn push_tines(entity: Cow<'_, Entity>, tines: &mut Vec<Tine>, source: &Arc<Source>) -> Result<(), Error> {
    match entity {
        Cow::Owned(Entity { term: Term::Train(owned), .. }) => tines.extend(owned),
        Cow::Borrowed(Entity { term: Term::Train(borrowed), .. }) => tines.extend_from_slice(borrowed),
        Cow::Owned(Entity { term: Term::Left(a, f), .. }) => tines.extend([Tine::Array(a), Tine::Function(f)]),
        Cow::Borrowed(Entity { term: Term::Left(a, f), .. }) => tines.extend([Tine::Array(a.clone()), Tine::Function(f.clone())]),
        entity if matches!(entity.category, Category::Value | Category::Callable) => tines.push(Tine::Array(array_of(entity, source)?.into_owned())),
        entity => tines.push(Tine::Function(function_of(entity, source)?.into_owned())),
    }
    Ok(())
}

/// Whether a call of `function` can return to the caller's loop as a tail call: the user defined it.
#[inline]
pub(super) fn tails(function: &Function) -> bool { matches!(function.node(), FunctionNode::Defined(_) | FunctionNode::Derived(..)) }

impl Category {
    /// The category of the field that `keys` reach from the array that a name's binding `found` holds, or of `found` itself where it holds no array, as `named` counts it. A field that the array doesn't have counts as an array.
    pub(super) fn field(found: Option<Binding>, keys: &[Arc<str>]) -> Self {
        match found {
            Some(Binding::Value(a)) => keys.iter().try_fold(a, |a, key| crate::keyed::field(&a, key)).map_or(Self::Value, |item| Self::of(&Binding::from_element(item))),
            found => Self::named(found.as_ref().map(Self::of)),
        }
    }
    pub(super) fn of(value: &Binding) -> Self {
        match value {
            Binding::NoResult => Self::NoResult, Binding::Absent => Self::Absent, Binding::Value(_) => Self::Value, Binding::Function(_) => Self::Function,
            Binding::Operator(op) => Self::of_operator(op),
        }
    }
    pub(super) fn of_operator(op: &self::OperatorNode) -> Self { if op.is_dyadic() { Self::DyadicOperator } else { Self::Operator } }
    /// A name's category from its binding's, as `Bindings::category` gives it: an unbound name counts as an array, and so does an absent `⍺`.
    pub(super) fn named(found: Option<Self>) -> Self { match found { None | Some(Self::Absent) => Self::Value, Some(category) => category } }
    /// Whether this is an operator's category, as `Some` of whether the operator is dyadic.
    pub(super) fn valence(self) -> Option<bool> { match self { Self::DyadicOperator => Some(true), Self::Operator => Some(false), _ => None } }
    /// The category of an array: `Callable` where it holds a function as an item, and otherwise `Value`.
    #[inline]
    pub(super) fn array(a: &Value) -> Self { if a.holds_functions() { Self::Callable } else { Self::Value } }
    /// The category that the binder and its recordings give `value`. Unlike `of`, it tells an array that holds a function from other arrays (`array`).
    #[inline]
    pub(super) fn bound(value: &Binding) -> Self { match value { Binding::Value(a) => Self::array(a), value => Self::of(value) } }
    /// How an item of this category binds before the item to its right, which binds as `right`, or with nothing to its right where `right` is `None`. An array that holds a function binds as a function before a subject or an absent `⍺`, and as an array otherwise. Every other category binds as itself.
    #[inline]
    pub(super) fn before(self, right: Option<Self>) -> Self {
        if self != Self::Callable { return self; }
        if matches!(right, Some(Self::Value | Self::Absent)) { Self::Function } else { Self::Value }
    }
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
    pub(super) fn evaluate(nodes: &[Node], session: &Session, tail: bool) -> Result<Step, Error> {
        Self::reduced(nodes, session, tail, None, None, None)?.step(&nodes[0].span.source)
    }
    /// `nodes` evaluated with `seed` already on the binder's stack, as the entity to the right of them all.
    pub(super) fn evaluate_seeded(nodes: &[Node], session: &Session, seed: Entity) -> Result<Bound, Error> { Self::seeded_with(nodes, session, seed, None) }
    /// `evaluate_seeded`, with `rec` recording the run's reductions. `rec` records the run as a level whose stack starts with the seed (`Recorder::seeded_starts`).
    pub(super) fn seeded_with(nodes: &[Node], session: &Session, seed: Entity, rec: Option<&mut Recorder>) -> Result<Bound, Error> {
        let Step::Done(bound) = Self::reduced(nodes, session, false, Some(seed), rec, None)?.step(&nodes[0].span.source)? else { unreachable!("a run with no tail call gives its value") };
        Ok(bound)
    }
    /// `nodes` evaluated as `evaluate` evaluates them, with the recording of the binder's steps when they stay within the subset that recordings cover (`tree.rs`).
    pub(super) fn recorded(nodes: &[Node], session: &Session, tail: bool) -> Result<(Step, Option<Recording>), Error> {
        let mut recorder = Recorder::default();
        let reduced = Self::reduced(nodes, session, tail, None, Some(&mut recorder), None)?;
        Ok((reduced.step(&nodes[0].span.source)?, recorder.finish()))
    }
    /// `nodes` evaluated from `resume`, the state where a recording's check failed.
    pub(super) fn resumed(nodes: &[Node], session: &Session, tail: bool, resume: Resume) -> Result<Step, Error> {
        Self::reduced(nodes, session, tail, None, None, Some(resume))?.step(&nodes[0].span.source)
    }
    /// The run or parenthesised group `node`, as the run around it uses it. A run reduces to one value: an array, a function, or an operator that a dot path such as `m.op` names.
    pub(super) fn nested(node: &Node, session: &Session) -> Result<Bound, Error> { Self::nested_with(node, session, None) }
    /// `nested`, with `rec` recording the group's reductions.
    fn nested_with(node: &Node, session: &Session, rec: Option<&mut Recorder>) -> Result<Bound, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        let Reduced::Entity(entity) = session.nested(nodes, |session| Self::reduced(nodes, session, false, None, rec, None))? else { unreachable!() };
        Self::group_bound(node, session, entity.bound(&nodes[0].span.source)?)
    }
    /// `bound`, the value of the group or run `node`, once checked: a run reduces to one value.
    pub(super) fn group_bound(node: &Node, session: &Session, bound: Bound) -> Result<Bound, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        if matches!(node.kind, NodeKind::Run(_)) && matches!(bound.value, Binding::Operator(_)) && !matches!(&session.members(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]) {
            return Err(Error::new(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph").at(&node.span));
        }
        Ok(bound)
    }
    /// The bracket literal `node`, at `cursor` among the nodes being walked, evaluated as `Session::resolve` evaluates it, each item as a statement of its own, while `rc` records each of its pieces as a level (`Recorder::piece`).
    fn literal(node: &Node, cursor: usize, session: &Session, rc: &mut Recorder) -> Result<Bound, Error> {
        rc.literal_starts(cursor);
        let value = session.bracket(node, |nodes, cell, at, within| {
            rc.piece_starts(cell, at);
            let Step::Done(bound) = session.nested(nodes, |session| Self::reduced(nodes, session, false, None, Some(&mut *rc), None)?.step(&nodes[0].span.source))? else { unreachable!("an item isn't in tail position") };
            let span = &nodes[0].span;
            let value = match within { super::session::Within::Key => Some(bound.array(span)?), _ => super::session::listed(bound.value, span)? };
            rc.piece(within, value.clone(), narrow(&crate::syntax::cover(nodes).range));
            Ok(value)
        })?;
        rc.literal(node, &value, session)?;
        Ok(Bound::new(value))
    }
    /// Pushes `entity`, working out the category it binds with against the entity below it (`Category::before`). Reductions take the top of the stack, so that entity stays below it while it's there.
    #[inline]
    fn push(&mut self, mut entity: Entity) {
        entity.role = entity.category.before(self.stack.last().map(|below| below.role));
        self.stack.push(entity);
    }
    /// A binder holding the `stack` that a recording resumes it with, with each entity's role worked out as `push` works it out.
    fn restacked(stack: Vec<Entity>) -> Self {
        let mut binder = Self { stack: Vec::with_capacity(stack.len()) };
        for entity in stack { binder.push(entity); }
        binder
    }
    /// Whether `left` goes onto `stack` now, rather than after the top of `stack` reduces.
    fn shifts(left: &Entity, stack: &[Entity]) -> bool {
        let [.., below, top] = stack else { return true };
        match Rule::get(top.role, below.role) {
            // An operator still awaiting its operand cannot reduce with its right neighbour.
            Rule::Wait(_) => true,
            // A superscript waits for its operand. On an array it gives an array, which can be the left argument of the function to its right.
            _ if matches!(&left.term, Term::Binding(Binding::Operator(self::OperatorNode::Primitive(OperatorKind::Super(_))))) => true,
            old => match Rule::get(left.category.before(Some(top.role)), top.role) {
                // Adjacent arrays group from the right: `v w i` is `v (w i)`.
                Rule::Adjacent if matches!(old, Rule::Adjacent) => false,
                new => new.strength() >= old.strength(),
            },
        }
    }
    fn reduced(
        nodes: &[Node], session: &Session, tail: bool, seed: Option<Entity>, mut rec: Option<&mut Recorder>, resume: Option<Resume>
    ) -> Result<Reduced, Error> {
        // A recording resumes the binder on the rewritten nodes it walked, whose positions its cursor counts.
        let rewriting = resume.is_none();
        let joined = if rewriting { session.joined(nodes, &mut rec.as_deref_mut().map(Recorder::facts)) } else { Cow::Borrowed(nodes) };
        let members = if rewriting { session.members_with(&joined, &mut rec.as_deref_mut().map(Recorder::facts)) } else { Cow::Borrowed(&joined[..]) };
        let nodes = &members[..];
        let source = &nodes[0].span.source;
        if matches!(joined, Cow::Owned(_)) || matches!(members, Cow::Owned(_)) { log(&mut rec, |rc| rc.rewritten(nodes.to_vec())); }
        let (mut binder, mut pending, mut cursor, mut held, mut after_call, mut assignment) = match resume {
            Some(r) => (Self::restacked(r.stack), r.pending, r.cursor, r.next, r.after_call, r.assignment),
            None => {
                let mut stacks = session.stacks.borrow_mut();
                (Self { stack: stacks.pop().unwrap_or_default() }, stacks.pop().unwrap_or_default(), nodes.len(), None, false, None)
            }
        };
        if let Some(seed) = seed { binder.push(seed); }
        loop {
            if !after_call {
                crate::execution::check().at(&nodes[0].span)?;
                let next = if let Some(entity) = held.take() { Some(entity) } else if pending.is_empty() && assignment.is_none() && cursor > 0 {
                    cursor -= 1;
                    let i = cursor;
                    let node = &nodes[i];
                    if matches!(node.kind, NodeKind::Assign) { assignment = Some(i); log(&mut rec, |rc| rc.assigning(i)); None } else {
                        let (term, expression) = if matches!(&node.kind, NodeKind::Run(inner) if session.ends_in_dyadic_operator(inner, &mut rec.as_deref_mut().map(Recorder::facts))) {
                            // A run that ends in a dyadic operator takes the next run as its right operand, because each run acts as one token.
                            let NodeKind::Run(inner) = &node.kind else { unreachable!() };
                            let Some(right) = binder.stack.pop() else {
                                return Err(Error::new(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it").at(&node.span));
                            };
                            // The run counts toward the depth limit, as a group does.
                            let bound = match rec.as_deref_mut() {
                                Some(rc) => {
                                    rc.seeded_starts(i);
                                    let bound = session.nested(inner, |session| Self::seeded_with(inner, session, right, Some(&mut *rc)))?;
                                    rc.group(node, &bound);
                                    bound
                                }
                                None => session.nested(inner, |session| Self::evaluate_seeded(inner, session, right))?,
                            };
                            (Term::Binding(bound.value), bound.expression)
                        } else if matches!(node.kind, NodeKind::Group(_) | NodeKind::Run(_)) {
                            let bound = match rec.as_deref_mut() {
                                Some(rc) => { rc.group_starts(i); let bound = Self::nested_with(node, session, Some(&mut *rc))?; rc.group(node, &bound); bound }
                                None => Self::nested(node, session)?,
                            };
                            (Term::Binding(bound.value), bound.expression)
                        } else if let (NodeKind::ArrayLiteral { .. }, Some(rc)) = (&node.kind, rec.as_deref_mut()) {
                            let bound = Self::literal(node, i, session, rc)?;
                            (Term::Binding(bound.value), bound.expression)
                        } else { let bound = session.resolve(node)?; log(&mut rec, |rc| rc.leaf(node, i, &bound.value, session)); (Term::Binding(bound.value), bound.expression) };
                        Some(Entity::new(term, narrow(&node.span.range), false, expression))
                    }
                } else { None };
                if let Some(left) = pending.last() {
                    if Self::shifts(left, &binder.stack) { binder.push(pending.pop().unwrap()); log(&mut rec, |rc| rc.unpend()); continue; }
                } else if let Some(left) = next {
                    if Self::shifts(&left, &binder.stack) { binder.push(left); log(&mut rec, |rc| rc.next(true)); continue; }
                    pending.push(left);
                    log(&mut rec, |rc| rc.next(false));
                } else if binder.stack.len() < 2 {
                    if let Some(i) = assignment.take() {
                        let begin = nodes[..i].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |j| j + 1);
                        if begin == i { return Err(Error::new(ErrorKind::Syntax, NO_TARGET).at(&nodes[i].span)); }
                        let range = narrow(&nodes[i].span.range);
                        // With nothing after `←`, `a(f)←` sets `a` to `f a` and gives no result.
                        let value = match binder.stack.pop() {
                            None => None,
                            Some(entity) => {
                                let value = entity.value(source)?;
                                if matches!(value, Binding::NoResult) { return Err(Error::new(ErrorKind::Value, "assignment requires a value").at(&nodes[i].span)); }
                                if matches!(value, Binding::Absent) { return Err(Error::new(ErrorKind::Value, ABSENT).at(&nodes[i].span)); }
                                Some(value)
                            }
                        };
                        let monadic = value.is_none();
                        cursor = begin + session.assignment_start(&nodes[begin..i])?;
                        let value = session.assigned(&nodes[cursor..i], value, None, &nodes[i].span)?;
                        log(&mut rec, |rc| {
                            let local = match &nodes[cursor..i] { [Node { kind: NodeKind::Name(name), .. }] if !monadic => match session.bindings.here(*name) { Place::Slot(_, slot) => Some(slot), _ => None }, _ => None };
                            rc.assigned(i - begin, cursor, monadic, local, range.clone())
                        });
                        pending.push(Entity::new(Term::Binding(value), range, true, None));
                        continue;
                    }
                    break;
                }
                }
            loop {
                let call = if std::mem::take(&mut after_call) { None } else { binder.reduce(session.capture.get(), source, rec.as_deref_mut())? };
                if let Some(call) = call {
                    // The binder's last reduction, with nothing left to resolve or on either stack and no assignment in progress. A dfn's last statement returns such a call of a function that the user defined to the caller's loop.
                    let last = assignment.is_none() && cursor == 0 && pending.is_empty() && binder.stack.is_empty();
                    if tail && last && tails(&call.function) {
                            log(&mut rec, |rc| rc.called(call.span.clone(), Category::Value, true));
                            session.stacks.borrow_mut().extend([binder.stack, pending]);
                            return Ok(Reduced::Tail(call));
                        }
                    let bound = call.function.call(call.left.as_ref(), &call.right, &call.span)?;
                    let entity = Entity::new(Term::Binding(bound.value), narrow(&call.span.range), bound.shy, call.expression);
                    log(&mut rec, |rc| rc.called(call.span.clone(), entity.category, last));
                    binder.push(entity);
                }
                // Bound operators still need their left operand before rebinding on the right.
                let m = binder.stack.len();
                if matches!(binder.stack[m - 1].category, Category::Operator) || Self::shifts(&binder.stack[m - 1], &binder.stack[..m - 1]) { break; }
                pending.push(binder.stack.pop().unwrap());
                log(&mut rec, |rc| rc.pend());
                crate::execution::check().at(&nodes[0].span)?;
            }
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        session.stacks.borrow_mut().extend([binder.stack, pending]);
        Ok(Reduced::Entity(entity))
    }
    fn reduce(&mut self, capture: bool, source: &Arc<Source>, mut rec: Option<&mut Recorder>) -> Result<Option<Application>, Error> {
        let mut left = self.stack.pop().unwrap();
        let mut right = self.stack.pop().unwrap();
        let range = left.range.clone();
        let span = || span_of(source, &range);
        let rule = Rule::get(left.role, right.role);
        if let Some(rc) = rec.as_deref_mut() {
            match rule {
                Rule::Drop => rc.reduce(Reduction::Drop),
                Rule::DropRight => rc.reduce(Reduction::DropRight),
                Rule::Adjacent => rc.reduce(Reduction::Strand),
                Rule::Attach => rc.reduce(Reduction::Attach),
                Rule::Call if !matches!(right.term, Term::Binding(Binding::Absent)) => rc.call = true,
                // A derive or a bind logs once its result is known.
                Rule::Derive | Rule::BindRight => {}
                Rule::Train => rc.reduce(Reduction::Train),
                _ => rc.fail(),
            }
        }
        let left_expression = left.expression.take();
        let right_expression = right.expression.take();
        let mut expression = None;
        let term = match rule {
            Rule::Missing => return Err(Error::new(ErrorKind::Value, "expression produced no value").at(&span())),
            Rule::Absent => return Err(Error::new(ErrorKind::Value, ABSENT).at(&span())),
            Rule::Drop => {
                right.expression = right_expression;
                self.push(right);
                return Ok(None);
            }
            Rule::DropRight => {
                left.expression = left_expression;
                self.push(left);
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
                    log(&mut rec, |rc| rc.reduce(Reduction::Derive { span: span(), range: range.clone(), result: Category::bound(&binding), value: binding.clone() }));
                    Term::Binding(binding)
                }
                // The call logs the derive once the binder has made it (`Recorder::called`).
                Derived::Call(function, x, y) => {
                    if let Some(rc) = rec.as_deref_mut() { (rc.call, rc.derive) = (true, true); }
                    let expression = capture.then(|| {
                        let operand = left_expression.map_or_else(|| Expression::array(x.as_ref().unwrap_or(&y)), |e| *e);
                        Box::new(if x.is_some() { Expression::call(&function, Some(operand), Expression::array(&y)) } else { Expression::call(&function, None, operand) })
                    });
                    return Ok(Some(Application { function, left: x, right: y, span: span(), unshy: false, expression }));
                }
            },
            Rule::BindRight => {
                let bound = bound_right(left.value(source)?, right.value(source)?);
                log(&mut rec, |rc| rc.reduce(Reduction::BindRight(bound.clone())));
                Term::Binding(Binding::Operator(bound))
            }
            Rule::Attach => {
                let (x, f, range) = attached(Cow::Owned(left), Cow::Owned(right), source)?;
                self.push(Entity::new(Term::Left(x.into_owned(), f.into_owned()), range, false, left_expression));
                return Ok(None);
            }
            Rule::Call if matches!(right.term, Term::Binding(Binding::Absent)) => Term::Binding(Binding::Absent),
            Rule::Call => {
                let (x, f) = split_call(Cow::Owned(left), source)?;
                let y = right.array(source)?;
                let expression = capture.then(|| {
                    Box::new(Expression::call(
                        &f, x.as_ref().map(|a| left_expression.map_or_else(|| Expression::array(a), |e| *e)),
                        right_expression.map_or_else(|| Expression::array(&y), |e| *e)))
                });
                return Ok(Some(Application { function: f.into_owned(), left: x.map(Cow::into_owned), right: y, span: span(), unshy: false, expression }));
            }
            Rule::Train => Term::Train(train(Cow::Owned(left), Cow::Owned(right), source)?),
            _ => return Err(Error::new(ErrorKind::Syntax, "these grammatical categories do not bind").at(&span())),
        };
        self.push(Entity::new(term, range, false, expression));
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
