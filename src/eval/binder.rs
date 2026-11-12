//! The binder: the right-to-left reduction of a run's items by their categories, and the rewriting of dot access.

use super::{ABSENT, Application, Binding, Bound, Function, FunctionNode, NO_TARGET, Operand, OperatorNode, Session, Step, dissect::Expression};
use crate::{
    At, Described, Error, ErrorKind, Source, Span, Value, primitive::{Builtin, OperatorKind, Primitive, Superscript},
    syntax::{ListForm, Node, NodeKind, ScriptKind}
};
use std::{borrow::Cow, sync::Arc};

fn stage_marker(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Operator(OperatorKind::Atop))
        || matches!(&node.kind, NodeKind::Embedded(Value::Operator(op)) if matches!(*op.0, OperatorNode::Primitive(OperatorKind::Atop)))
}

impl Session {
    /// Lower selector sections and data-first operator tails before runs bind their operands.
    fn sections<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        let dot = |n: &Node| matches!(n.kind, NodeKind::Operator(OperatorKind::Product));
        let bracket = |n: &Node| n.span.source.text.as_bytes().get(n.span.range.start) == Some(&b'[');
        let contains = |test: &dyn Fn(&Node) -> bool| {
            nodes.iter().any(|n| match &n.kind { NodeKind::Run(inner) => inner.iter().any(test), _ => test(n) })
        };
        if !contains(&dot) || !contains(&bracket) { return Cow::Borrowed(nodes); }
        let flat: Vec<_> = nodes.iter().flat_map(|n| match &n.kind { NodeKind::Run(inner) => &inner[..], _ => std::slice::from_ref(n) }).cloned().collect();
        let group = |nodes: Vec<Node>| Node { span: crate::syntax::cover(&nodes), kind: NodeKind::Group(nodes) };
        let (mut out, mut i, mut changed) = (Vec::with_capacity(flat.len()), 0, false);
        while i < flat.len() {
            let Some(head) = flat.get(i + 1).filter(|_| dot(&flat[i])) else {
                out.push(flat[i].clone());
                i += 1;
                continue;
            };
            let immediate = bracket(head);
            let explicit = !crate::syntax::touching(&flat[i], head) || matches!(head.kind, NodeKind::Operator(_) | NodeKind::Script(ScriptKind::Super, _));
            if !immediate && !(explicit && self.starts_with_operator(&flat[i + 1..])) {
                out.push(flat[i].clone());
                i += 1;
                continue;
            }
            let Some(end) = flat[i + 1..].iter().take_while(|n| !matches!(n.kind, NodeKind::Assign)).position(bracket).map(|j| i + 1 + j) else {
                out.push(flat[i].clone());
                i += 1;
                continue;
            };
            let prefix = self.members_with(&out);
            let subject = prefix.len().checked_sub(1).and_then(|mut start| {
                if matches!(prefix[start].kind, NodeKind::Assign) { return None; }
                while start > 0 && self.static_category(&prefix[start]) == Category::Operator && crate::syntax::touching(&prefix[start - 1], &prefix[start]) {
                    start -= 1;
                }
                self.trailing_operand(&prefix[start..], prefix.len() - start).ok().filter(|(_, c)| matches!(c, Category::Value | Category::Function))
                    .map(|(_, c)| (start, c))
            });
            if immediate && subject.is_some() {
                out.push(flat[i].clone());
                i += 1;
                continue;
            }
            out = prefix.into_owned();
            let root = subject.map(|(start, category)| (group(out.split_off(start)), category));
            let identity = Node { kind: NodeKind::Function(Builtin::Primitive(Primitive::Identity(false))), span: flat[i].span.clone() };
            let selector = group(vec![identity, flat[i].clone(), flat[end].clone()]);
            let modified = group(crate::syntax::runs(std::iter::once(selector).chain(flat[i + 1..end].iter().cloned()).collect()));
            let mut lowered = vec![modified];
            if let Some((root, category)) = root {
                if category == Category::Function { lowered.push(Node { kind: NodeKind::Operator(OperatorKind::Atop), span: flat[i].span.clone() }); }
                lowered.push(root);
            }
            crate::syntax::reorder_reads(&mut lowered);
            out.push(group(lowered));
            (i, changed) = (end + 1, true);
        }
        if changed { Cow::Owned(crate::syntax::runs(out)) } else { Cow::Borrowed(nodes) }
    }

    fn has_members(nodes: &[Node]) -> bool {
        nodes.iter().any(|n| matches!(n.kind, NodeKind::Script(ScriptKind::Sub, _) | NodeKind::Joiner(_))) || nodes.windows(2).any(|w| {
            matches!(w[0].kind, NodeKind::Operator(OperatorKind::Product))
                && matches!(w[1].kind, NodeKind::Name(_, _) | NodeKind::Group(_) | NodeKind::ArrayLiteral(_))
        })
    }

    /// Rewrite a writable target's dot reads and numeric subscripts to selection calls.
    pub(super) fn selection_members<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        let members = match self.sections(nodes) {
            Cow::Borrowed(nodes) => self.members_with(nodes), Cow::Owned(nodes) => Cow::Owned(self.members_with(&nodes).into_owned())
        };
        let Some(last) = members.last() else { return members };
        match &last.kind {
            NodeKind::Group(inner) | NodeKind::Run(inner) => {
                let inner = self.selection_members(inner);
                if matches!(inner, Cow::Borrowed(_)) { return members; }
                let kind = if matches!(last.kind, NodeKind::Run(_)) { NodeKind::Run(inner.into_owned()) } else { NodeKind::Group(inner.into_owned()) };
                let mut out = members.into_owned();
                out.last_mut().unwrap().kind = kind;
                Cow::Owned(out)
            }
            NodeKind::Script(ScriptKind::Sub, index) => {
                let index = Node { kind: NodeKind::Literal(index.clone()), span: last.span.clone() };
                let Ok((start, _)) = self.trailing_operand(&members, members.len() - 1) else { return members };
                let mut out = members.into_owned();
                let subscript = out.pop().unwrap();
                let inner = out.split_off(start);
                let span = crate::syntax::cover(&inner);
                let inner = self.selection_members(&inner).into_owned();
                let item = if inner.len() == 1 { inner.into_iter().next().unwrap() } else { Node { kind: NodeKind::Group(inner), span } };
                let span = Span { source: item.span.source.clone(), range: item.span.range.start..subscript.span.range.end };
                out.push(Node {
                    kind: NodeKind::Group(vec![index, Node { kind: NodeKind::Function(Builtin::Primitive(Primitive::Index)), span: subscript.span }, item]),
                    span });
                Cow::Owned(out)
            }
            _ => members,
        }
    }
    /// Finish the current run before an operator: `⊂⊢∘⌸` is `(⊂⊢)⌸`. Names and dot paths use their resolved categories.
    fn stages<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        let boundary = |i: usize| stage_marker(&nodes[i]) && nodes.get(i + 1).is_some_and(|next| self.static_category(next).valence().is_some());
        if !(1..nodes.len()).any(&boundary) { return Cow::Borrowed(nodes); }
        let (mut out, mut start) = (Vec::with_capacity(nodes.len()), 0);
        for (i, node) in nodes.iter().enumerate() {
            // Selector lowering can flatten runs; an operator still takes the run before it as its operand.
            if i > 0 && !crate::syntax::touching(&nodes[i - 1], node) && self.static_category(node).valence().is_none() { start = out.len(); }
            if boundary(i) && out.len() > start {
                let inner = out.split_off(start);
                out.push(Node { span: crate::syntax::cover(&inner), kind: NodeKind::Group(inner) });
            } else { out.push(node.clone()); if matches!(node.kind, NodeKind::Assign) { start = out.len(); } }
        }
        Cow::Owned(out)
    }

    /// Rewrite dot reads and operator-stage boundaries.
    pub(super) fn members_with<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        if !Self::has_members(nodes) { return self.stages(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        // Whether the last node in `out` is the right operand of a `.` after a function.
        let (mut i, mut on_function) = (0, false);
        while i < nodes.len() {
            if (matches!(nodes[i].kind, NodeKind::Script(ScriptKind::Sub, _)) && on_function
                || matches!(nodes[i].kind, NodeKind::Operator(OperatorKind::Product))
                    && matches!(out.last().map(|n| &n.kind), Some(NodeKind::Script(ScriptKind::Sub, _))))
                && let Ok((start, _)) = self.trailing_operand(&out, out.len()) {
                    let inner = out.split_off(start);
                    let span = crate::syntax::cover(&inner);
                    out.push(Node { kind: NodeKind::Group(inner), span });
                    on_function = false;
                }
            if let (NodeKind::Operator(OperatorKind::Product), Some(next)) = (&nodes[i].kind, nodes.get(i + 1)) {
                let root = out.last().filter(|root| {
                    !on_function && match &root.kind {
                        NodeKind::Name(_, _) | NodeKind::Group(_) | NodeKind::Embedded(_) => self.holds_array(root),
                        NodeKind::Literal(_) | NodeKind::ArrayLiteral(_) | NodeKind::Scope(..) => true, _ => false
                    }
                });
                // `.name` reads a field and `.[I]` selects, after an array or a function. After a function, a name that starts a dot path to a function is an inner product's operand instead, as in `+.ops.mul`.
                let read = match &next.kind {
                    NodeKind::Name(name, _) if root.is_some() || !self.path_function(&nodes[i + 1..]) => {
                        Some((Node { kind: NodeKind::Literal(crate::keyed::text(name.text())), span: next.span.clone() }, Primitive::Mix))
                    }
                    NodeKind::ArrayLiteral(literal) if matches!(literal.form, ListForm::Items | ListForm::Cells) => Some((next.clone(), Primitive::Bracket)),
                    _ => None,
                };
                if let Some(root) = root {
                    let kind = match (read, &next.kind) {
                        (Some((key, function)), _) => Some(NodeKind::Group(vec![
                            key, Node { kind: NodeKind::Function(Builtin::Primitive(function)), span: nodes[i].span.clone() }, root.clone(),
                        ])),
                        (None, kind)
                            if matches!(kind, NodeKind::Group(_)) || matches!(kind, NodeKind::ArrayLiteral(literal) if literal.form == ListForm::Rows) =>
                        { Some(NodeKind::Scope(Box::new(root.clone()), Box::new(next.clone()))) }
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
            out.push(nodes[i].clone());
            i += 1;
        }
        // `ⱼ`, `ₑ` or `ᵣ` joins the items it touches once they're bound, as one group (`syntax::joined_term`): `v₁ⱼT.b` joins `1⌷v` and `'b'⊃T`. The left item takes the superscripts it touches, as a literal does, so `x²ⱼy` joins `x²` and `y`. `ₑ` joins first, as in the literal `1ⱼ2ₑ3`, which is `1ⱼ2000`. An `ᵣ` that joins nothing becomes the mark `ᵣ`. An `ⱼ` or `ₑ` beside a function, or apart from an item, stays for `resolve` to reject.
        use crate::syntax::{Joiner, cover, joined_term, touching};
        for exponent in [true, false] {
            let mut i = 1;
            while i + 1 < out.len() {
                let NodeKind::Joiner(joiner) = out[i].kind else { i += 1; continue; };
                let mut start = i - 1;
                while start > 0 && matches!(out[start].kind, NodeKind::Operator(OperatorKind::Super(_)) | NodeKind::Script(..))
                    && touching(&out[start - 1], &out[start]) { start -= 1; }
                let left =
                    if start + 1 == i { out[start].clone() } else { Node { kind: NodeKind::Group(out[start..i].to_vec()), span: cover(&out[start..i]) } };
                if matches!(joiner, Joiner::Exponent) != exponent || !touching(&left, &out[i]) || !touching(&out[i], &out[i + 1]) || !self.holds_array(&left)
                    || !self.holds_array(&out[i + 1]) { i += 1; continue; }
                let joined = joined_term(left, joiner, &out[i].span, out[i + 1].clone());
                out.splice(start..i + 2, [joined]);
                i = start + 1;
            }
        }
        // An `ᵣ` that joins nothing marks what it follows, as `÷ᵣ` divides in the unbounded domain.
        for node in &mut out {
            if matches!(node.kind, NodeKind::Joiner(Joiner::Denominator)) {
                node.kind = NodeKind::Operator(OperatorKind::Super(Superscript::Mark(crate::number::Mark::Rational)));
            }
        }
        match self.stages(&out) { Cow::Borrowed(_) => Cow::Owned(out), Cow::Owned(staged) => Cow::Owned(staged) }
    }

    /// Whether a run ends in a dyadic operator, which then takes the next run as its right operand.
    pub(super) fn ends_in_dyadic_operator(&self, nodes: &[Node]) -> bool {
        self.members_with(nodes).last().is_some_and(|n| self.static_category(n).valence() == Some(true))
    }

    /// Whether a run starts with an operator, which then takes the whole run before it as its left operand.
    pub(super) fn starts_with_operator(&self, nodes: &[Node]) -> bool {
        self.members_with(nodes).first().is_some_and(|n| self.static_category(n).valence().is_some())
    }

    /// Join each run that starts with an operator to the node before it, as the run's first token: `≢⊢ ⌸x` reads as `(≢⊢)⌸x`. A run before it joins as a group. A joined run that still starts with an operator joins again: `+ / ¨x` reads as `+/¨x`. No join reaches across `←`.
    fn joined<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        let starts = |node: &Node| matches!(&node.kind, NodeKind::Run(inner) if self.starts_with_operator(inner));
        let joins = |before: &Node, node: &Node| {
            starts(node)
                || matches!(&before.kind, NodeKind::Run(inner) if inner.last().is_some_and(stage_marker)) && self.static_category(node).valence().is_some()
        };
        if !nodes.windows(2).any(|w| joins(&w[0], &w[1])) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        for node in nodes {
            let mut node = node.clone();
            while out.last().is_some_and(|before| !matches!(before.kind, NodeKind::Assign) && joins(before, &node)) {
                let before = out.pop().expect("a node is before this run");
                let span = Span { source: before.span.source.clone(), range: before.span.range.start..node.span.range.end };
                let before = match before.kind {
                    NodeKind::Run(mut run) => {
                        if run.last().is_some_and(stage_marker) { run.pop(); }
                        Node { span: crate::syntax::cover(&run), kind: NodeKind::Group(run) }
                    }
                    _ => before,
                };
                let inner = match node.kind { NodeKind::Run(inner) => inner, _ => vec![node] };
                node = Node { kind: NodeKind::Run(std::iter::once(before).chain(inner).collect()), span };
            }
            out.push(node);
        }
        if let [Node { kind: NodeKind::Run(inner), .. }] = out.as_mut_slice() { return Cow::Owned(std::mem::take(inner)); }
        Cow::Owned(out)
    }
    /// Whether the dot path that starts `nodes`, such as `mul` or `ops.mul`, names a function or operator, read without evaluating anything (`path_category`).
    fn path_function(&self, nodes: &[Node]) -> bool {
        let Some(NodeKind::Name(name, _)) = nodes.first().map(|n| &n.kind) else { return false };
        let keys: Vec<Arc<str>> = nodes[1..].chunks_exact(2).map_while(|pair| match pair {
            [Node { kind: NodeKind::Operator(OperatorKind::Product), .. }, Node { kind: NodeKind::Name(key, _), .. }] => Some(key.text().into()), _ => None
        }).collect();
        self.path_category(*name, &keys) != Category::Value
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
}
#[derive(Clone, Debug)]
pub(super) enum Tine { Array(Value), Function(Function) }
/// The last entity of a run, or the call it ends in when that call is in tail position.
enum Reduced { Entity(Entity), Tail(Application) }
impl Reduced {
    /// The step that a run gives: its call in tail position, or its last entity's value.
    fn step(self, source: &Arc<Source>) -> Result<Step, Error> {
        Ok(match self { Self::Tail(call) => Step::Tail(call), Self::Entity(entity) => Step::Done(entity.bound(source)?) })
    }
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
const _: () = assert!(std::mem::size_of::<Entity>() <= 64);

/// `range` as `u32` offsets, clamped for a source longer than 4 GiB.
pub(super) fn narrow(range: &std::ops::Range<usize>) -> std::ops::Range<u32> {
    let at = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
    at(range.start)..at(range.end)
}
/// The span of `range` in `source`.
pub(super) fn span_of(source: &Arc<Source>, range: &std::ops::Range<u32>) -> Span {
    Span { source: source.clone(), range: range.start as usize..range.end as usize }
}

impl Entity {
    /// An entity with `term`, not yet on the binder's stack.
    // Out of line, `new` built each entity and then copied it out, about 265 instructions for each call of `{x←ω+1 ◇ x}`.
    #[inline(always)]
    pub(super) fn new(term: Term, range: std::ops::Range<u32>, shy: bool, expression: Option<Box<Expression>>) -> Self {
        let category = match term {
            Term::Binding(ref value) => Category::bound(value), Term::Strand(_) => Category::Value, Term::Train(_) => Category::Train,
            Term::Left(..) => Category::Left
        };
        Self { term, range, shy, category, role: category, expression }
    }
    /// An entity that holds `bound`'s value and binds as it does (`Bound::category`).
    #[inline(always)]
    pub(super) fn held(bound: Bound, range: std::ops::Range<u32>) -> Self {
        let category = bound.category();
        Self { term: Term::Binding(bound.value), range, shy: bound.shy, category, role: category, expression: bound.expression }
    }
    /// The entity's value: its binding, or what its unfinished term builds (`Entity::built`).
    #[inline(always)]
    pub(super) fn value(self, source: &Arc<Source>) -> Result<Binding, Error> {
        match self.term { Term::Binding(value) => Ok(value), term => (Self { term, ..self }).built(source) }
    }
    /// What an unfinished term builds: a train's function, a bound left argument's function or a strand's vector.
    // Out of line, so that the conversions that every operand goes through stay small enough to inline.
    #[inline(never)]
    fn built(self, source: &Arc<Source>) -> Result<Binding, Error> {
        let span = || span_of(source, &self.range);
        Ok(match self.term {
            Term::Binding(value) => value, Term::Train(tines) => Binding::Function(self::Function::train(tines, &span())?),
            Term::Left(a, f) => Binding::Function(before(a, f, &span())?),
            Term::Strand(items) => Binding::Value(Value::new(vec![items.len()], items).map_err(|k| Error::new(k, "invalid strand").at(&span()))?)
        })
    }
    #[inline(always)]
    pub(super) fn function(self, source: &Arc<Source>) -> Result<Function, Error> {
        let range = self.range.clone();
        match self.value(source)? { Binding::Function(f) => Ok(f), value => Function::from_value(value, &span_of(source, &range)) }
    }
    #[inline(always)]
    pub(super) fn array(self, source: &Arc<Source>) -> Result<Value, Error> { match self.value(source)? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
    /// The entity's value, with whether it displays, the expression it captured and the category it binds with: its own, except that a train or a left argument with its function builds a function, which binds as a function.
    pub(super) fn bound(mut self, source: &Arc<Source>) -> Result<Bound, Error> {
        let category = (!matches!(self.term, Term::Train(_) | Term::Left(..))).then_some(self.category);
        let (shy, expression) = (self.shy, self.expression.take());
        Ok(Bound { value: self.value(source)?, shy, expression, category })
    }
    /// This entity as the value of the group `node`. A field read from a name (`dot_path`) binds as its field, as `Session::static_category` reads it, though its `⊃` call gave an array (`Bound::given`). Any other group binds as its entity did.
    pub(super) fn grouped(self, node: &Node) -> Self {
        if !matches!(self.term, Term::Binding(Binding::Value(_))) || super::assign::dot_path(node).is_none() { return self; }
        let Term::Binding(Binding::Value(a)) = self.term else { unreachable!() };
        Self::new(Term::Binding(Binding::from_element(a)), self.range, self.shy, self.expression)
    }
}

/// `left` and `right` side by side as a strand. A strand adds its items, and anything else adds itself as one array. Gives the items, how many of them came from `left`, and the strand's text range, which is `left`'s.
#[inline]
pub(super) fn strand(left: Entity, right: Entity, source: &Arc<Source>) -> Result<(Vec<Value>, usize, std::ops::Range<u32>), Error> {
    let range = left.range.clone();
    let mut joined = match left.term {
        Term::Strand(items) => items,
        term => {
            let mut items = Vec::with_capacity(2);
            items.push(Entity { term, ..left }.array(source)?);
            items
        }
    };
    let from_left = joined.len();
    match right.term { Term::Strand(items) => joined.extend(items), term => joined.push(Entity { term, ..right }.array(source)?) }
    Ok((joined, from_left, range))
}

/// Bind `left` to `right`; the call's source range belongs to the function, not the bound argument.
#[inline]
fn attached(left: Entity, right: Entity, source: &Arc<Source>) -> Result<(Value, Function, std::ops::Range<u32>), Error> {
    let range = right.range.clone();
    Ok((left.array(source)?, right.function(source)?, range))
}

/// Separate a call's function and optional bound left argument from the entity before its right argument.
#[inline]
fn split_call(left: Entity, source: &Arc<Source>) -> Result<(Option<Value>, Function), Error> {
    Ok(match left { Entity { term: Term::Left(x, f), .. } => (Some(x), f), left => (None, left.function(source)?) })
}

/// What the operator `operator` gives on the operand `operand` (`Rule::Derive`): a derived function, a scalar that holds the function for `ᵘ`, or the call that a power, `ᵀ` or a number marker on an array stands for: of `*`, `⍉`, `⌊ₓ` or `•float`.
pub(super) enum Derived { Term(Binding), Call(Function, Option<Value>, Value) }

/// `operand` given to `operator` at `span`, as `Derived` describes.
pub(super) fn derived(operand: Binding, operator: Binding, span: &Span) -> Result<Derived, Error> {
    let operand = Operand::from_value(operand);
    let Binding::Operator(operator) = operator else { unreachable!("a derive's right item is an operator") };
    let function = |p| self::Function::primitive(p);
    if let (self::OperatorNode::Script(kind, indices), Operand::Value(a)) = (&operator, &operand) {
        if *kind == ScriptKind::Sub { return Ok(Derived::Call(function(Primitive::Index), Some(indices.clone()), a.clone())); }
        let powers = indices.as_items().integers().described("invalid superscript powers").at(span)?;
        let power = function(Primitive::Math(crate::number::Math::Power));
        if powers.len() == 1 { return Ok(Derived::Call(power, Some(a.clone()), Value::weak_integer(powers[0]))); }
        // On data, spaced superscripts retain successive powers; on functions they are Rank's one operand.
        let mut chain = None;
        for &n in powers.iter() {
            let next =
                Function::new(FunctionNode::Composed(OperatorKind::After, [Operand::Function(power.clone()), Operand::Value(Value::weak_integer(n))]), span)?;
            chain = Some(match chain {
                None => next, Some(f) => Function::new(FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(next), Operand::Function(f)]), span)?
            });
        }
        return Ok(Derived::Call(chain.expect("a numeric script has at least one integer"), None, a.clone()));
    }
    if let self::OperatorNode::Primitive(OperatorKind::Super(s)) = &operator {
        match (*s, operand) {
            // `ᵘ` makes a scalar that holds a function. `⊂` encloses a subject.
            (Superscript::Unit, Operand::Function(f)) => {
                return Ok(Derived::Term(Binding::Value(crate::Value::Function(f).enclose().described("invalid scalar").at(span)?)));
            }
            (Superscript::Unit, _) => return Err(Error::domain("ᵘ makes a scalar that holds a function. Enclose a subject with ⊂").at(span)),
            (Superscript::Transpose, Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Transpose), None, a)),
            (Superscript::Mark(mark), Operand::Value(a)) => return Ok(Derived::Term(Binding::Value(super::operators::converted_by(&a, mark, span)?))),
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

/// The tines of `left` and then `right`, reusing either unfinished train's list.
#[inline]
fn train(left: Entity, right: Entity, source: &Arc<Source>) -> Result<Vec<Tine>, Error> {
    Ok(match (left, right) {
        (Entity { term: Term::Train(mut tines), .. }, right) => { push_tines(right, &mut tines, source)?; tines }
        (left, Entity { term: Term::Train(mut tines), .. }) => {
            let n = tines.len();
            push_tines(left, &mut tines, source)?;
            let pushed = tines.len() - n;
            tines.rotate_right(pushed);
            tines
        }
        (left, right) => {
            let mut tines = Vec::with_capacity(4);
            push_tines(left, &mut tines, source)?;
            push_tines(right, &mut tines, source)?;
            tines
        }
    })
}

/// Append a train's tines, flattening unfinished trains and preserving a bound left argument before its function.
#[inline]
fn push_tines(entity: Entity, tines: &mut Vec<Tine>, source: &Arc<Source>) -> Result<(), Error> {
    match entity {
        Entity { term: Term::Train(owned), .. } => tines.extend(owned),
        Entity { term: Term::Left(a, f), .. } => tines.extend([Tine::Array(a), Tine::Function(f)]),
        entity if matches!(entity.category, Category::Value | Category::Callable) => tines.push(Tine::Array(entity.array(source)?)),
        entity => tines.push(Tine::Function(entity.function(source)?)),
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
            Some(Binding::Value(a)) => {
                keys.iter().try_fold(a, |a, key| crate::keyed::field(&a, key)).map_or(Self::Value, |item| Self::of(&Binding::from_element(item)))
            }
            found => Self::named(found.as_ref().map(Self::of)),
        }
    }
    pub(super) fn of(value: &Binding) -> Self {
        match value {
            Binding::NoResult => Self::NoResult, Binding::Absent => Self::Absent, Binding::Value(_) | Binding::Released => Self::Value,
            Binding::Function(_) => Self::Function, Binding::Operator(op) => Self::of_operator(op)
        }
    }
    pub(super) fn of_operator(op: &self::OperatorNode) -> Self { if op.is_dyadic() { Self::DyadicOperator } else { Self::Operator } }
    /// A name's category from its binding's, as `Bindings::category` gives it: an unbound name counts as an array, and so does an absent `α`.
    pub(super) fn named(found: Option<Self>) -> Self { match found { None | Some(Self::Absent) => Self::Value, Some(category) => category } }
    /// Whether this is an operator's category, as `Some` of whether the operator is dyadic.
    pub(super) fn valence(self) -> Option<bool> { match self { Self::DyadicOperator => Some(true), Self::Operator => Some(false), _ => None } }
    /// The category of an array: `Callable` where it holds a function as an item, and otherwise `Value`.
    #[inline]
    pub(super) fn array(a: &Value) -> Self { if a.holds_functions() { Self::Callable } else { Self::Value } }
    /// The category that the binder gives `value`. Unlike `of`, it tells an array that holds a function from other arrays (`array`).
    #[inline]
    pub(super) fn bound(value: &Binding) -> Self { match value { Binding::Value(a) => Self::array(a), value => Self::of(value) } }
    /// How an item of this category binds before the item to its right, which binds as `right`, or with nothing to its right where `right` is `None`. An array that holds a function binds as a function before a subject or an absent `α`, and as an array otherwise. Every other category binds as itself.
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
            // An absent `α` drops out: `α f ω` calls `f` monadically, `f α` is absent too, and `α ω` is `ω`.
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
        Self::reduced(nodes, session, tail, None)?.step(&nodes[0].span.source)
    }
    /// `nodes` evaluated with `seed` already on the binder's stack, as the entity to the right of them all.
    pub(super) fn evaluate_seeded(nodes: &[Node], session: &Session, seed: Entity) -> Result<Bound, Error> {
        let Step::Done(bound) = Self::reduced(nodes, session, false, Some(seed))?.step(&nodes[0].span.source)? else {
            unreachable!("a run with no tail call gives its value")
        };
        Ok(bound)
    }
    /// The run or parenthesised group `node`, as the run around it binds it. A run reduces to one value: an array, a function, or an operator that a dot path such as `m.op` names. The group binds as its last entity did, except a field read from a name (`Entity::grouped`).
    pub(super) fn nested(node: &Node, session: &Session) -> Result<Entity, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        let Reduced::Entity(entity) = session.nested(nodes, |session| Self::reduced(nodes, session, false, None))? else { unreachable!() };
        let bound = Self::group_bound(node, session, entity.bound(&nodes[0].span.source)?)?;
        Ok(Entity::held(Bound { shy: false, ..bound }, narrow(&node.span.range)).grouped(node))
    }
    /// `bound`, the value of the group or run `node`, once checked: a run reduces to one value.
    pub(super) fn group_bound(node: &Node, session: &Session, bound: Bound) -> Result<Bound, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        if matches!(node.kind, NodeKind::Run(_)) && matches!(bound.value, Binding::Operator(_))
            && !matches!(&session.members_with(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]) {
                return Err(Error::new(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph").at(&node.span));
            }
        Ok(bound)
    }
    /// Pushes `entity`, working out the category it binds with against the entity below it (`Category::before`). Reductions take the top of the stack, so that entity stays below it while it's there.
    #[inline]
    fn push(&mut self, mut entity: Entity) {
        entity.role = entity.category.before(self.stack.last().map(|below| below.role));
        self.stack.push(entity);
    }
    /// Whether `left` goes onto `stack` now, rather than after the top of `stack` reduces.
    fn shifts(left: &Entity, stack: &[Entity]) -> bool {
        let [.., below, top] = stack else { return true };
        match Rule::get(top.role, below.role) {
            // An operator still awaiting its operand cannot reduce with its right neighbour.
            Rule::Wait(_) => true,
            // Postfix operators bind an array before its neighbours use it as an argument or operand.
            _ if matches!(
                &left.term, Term::Binding(Binding::Operator(self::OperatorNode::Primitive(OperatorKind::Super(_)) | self::OperatorNode::Script(..)))
            ) =>
            { true }
            _ if top.role == Category::Value && matches!(
                &below.term, Term::Binding(Binding::Operator(self::OperatorNode::Primitive(OperatorKind::Super(_)) | self::OperatorNode::Script(..)))
            ) =>
            { false }
            old => match Rule::get(left.category.before(Some(top.role)), top.role) {
                // Adjacent arrays group from the right: `v w i` is `v (w i)`.
                Rule::Adjacent if matches!(old, Rule::Adjacent) => false,
                new => new.strength() >= old.strength(),
            },
        }
    }
    fn reduced(nodes: &[Node], session: &Session, tail: bool, seed: Option<Entity>) -> Result<Reduced, Error> {
        let sections = session.sections(nodes);
        let joined = session.joined(&sections);
        let members = session.members_with(&joined);
        let combined = super::rewrite::expression(session, &members);
        let nodes = &combined[..];
        let source = &nodes[0].span.source;
        let (mut binder, mut pending) = {
            let mut stacks = session.stacks.borrow_mut();
            (Self { stack: stacks.pop().unwrap_or_default() }, stacks.pop().unwrap_or_default())
        };
        let (mut cursor, mut assignment) = (nodes.len(), None);
        if let Some(seed) = seed { binder.push(seed); }
        loop {
            crate::cancel::check().at(&nodes[0].span)?;
            let next = if pending.is_empty() && assignment.is_none() && cursor > 0 {
                cursor -= 1;
                let i = cursor;
                let node = &nodes[i];
                if matches!(node.kind, NodeKind::Assign) { assignment = Some(i); None } else {
                    let range = narrow(&node.span.range);
                    // Each run acts as one token; a trailing dyadic operator takes the next run as its right operand.
                    let entity = if matches!(&node.kind, NodeKind::Run(inner) if session.ends_in_dyadic_operator(inner)) {
                        let NodeKind::Run(inner) = &node.kind else { unreachable!() };
                        let Some(right) = binder.stack.pop() else {
                            return Err(Error::new(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it").at(&node.span));
                        };
                        // The run counts toward the depth limit, like a parenthesized group.
                        Entity::held(Bound { shy: false, ..session.nested(inner, |session| Self::evaluate_seeded(inner, session, right))? }, range)
                    } else if matches!(node.kind, NodeKind::Group(_) | NodeKind::Run(_)) { Self::nested(node, session)? }
                    else { Entity::held(Bound { shy: false, ..session.resolve(node)? }, range) };
                    Some(entity)
                }
            } else { None };
            if let Some(left) = pending.last() {
                if Self::shifts(left, &binder.stack) { binder.push(pending.pop().unwrap()); continue; }
            } else if let Some(left) = next {
                if Self::shifts(&left, &binder.stack) { binder.push(left); continue; }
                pending.push(left);
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
                            if matches!(value, Binding::NoResult) {
                                return Err(Error::new(ErrorKind::Value, "assignment requires a value").at(&nodes[i].span));
                            }
                            if matches!(value, Binding::Absent) { return Err(Error::new(ErrorKind::Value, ABSENT).at(&nodes[i].span)); }
                            Some(value)
                        }
                    };
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    let value = session.assigned(&nodes[cursor..i], value, &nodes[i].span)?;
                    pending.push(Entity::new(Term::Binding(value), range, true, None));
                    continue;
                }
                break;
            }
            loop {
                if let Some(call) = binder.reduce(session.capture.get(), source)? {
                    // A dfn's final user-defined call returns to the caller's loop instead of adding a Rust frame.
                    let last = assignment.is_none() && cursor == 0 && pending.is_empty() && binder.stack.is_empty();
                    if tail && last && tails(&call.function) { session.stacks.borrow_mut().extend([binder.stack, pending]); return Ok(Reduced::Tail(call)); }
                    let bound = call.function.call_with(call.left.map(Cow::Owned), Cow::Owned(call.right), &call.span)?;
                    let entity = Entity::held(Bound { expression: call.expression, ..bound.given() }, narrow(&call.span.range));
                    binder.push(entity);
                }
                // A bound operator still needs its left operand before rebinding on the right.
                let m = binder.stack.len();
                if matches!(binder.stack[m - 1].category, Category::Operator) || Self::shifts(&binder.stack[m - 1], &binder.stack[..m - 1]) { break; }
                pending.push(binder.stack.pop().unwrap());
                crate::cancel::check().at(&nodes[0].span)?;
            }
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        session.stacks.borrow_mut().extend([binder.stack, pending]);
        Ok(Reduced::Entity(entity))
    }
    // A browser worker's small stack limits recursion. In wasm, inlining this into `reduced` would put its locals in the frame that `reduced` keeps across each call, at every level of recursion. V8's Liftoff gives each local a slot, whether or not its path runs.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    fn reduce(&mut self, capture: bool, source: &Arc<Source>) -> Result<Option<Application>, Error> {
        let mut left = self.stack.pop().unwrap();
        let mut right = self.stack.pop().unwrap();
        let range = left.range.clone();
        let span = || span_of(source, &range);
        let rule = Rule::get(left.role, right.role);
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
                Derived::Term(binding) => Term::Binding(binding),
                Derived::Call(function, x, y) => {
                    let expression = capture.then(|| {
                        let operand = left_expression.map_or_else(|| Expression::array(x.as_ref().unwrap_or(&y)), |e| *e);
                        Box::new(if x.is_some() { Expression::call(&function, Some(operand), Expression::array(&y)) }
                        else { Expression::call(&function, None, operand) })
                    });
                    return Ok(Some(Application { function, left: x, right: y, span: span(), unshy: false, expression }));
                }
            },
            Rule::BindRight => { let bound = bound_right(left.value(source)?, right.value(source)?); Term::Binding(Binding::Operator(bound)) }
            Rule::Attach => {
                let (x, f, range) = attached(left, right, source)?;
                self.push(Entity::new(Term::Left(x, f), range, false, left_expression));
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
                return Ok(Some(Application { function: f, left: x, right: y, span: span(), unshy: false, expression }));
            }
            Rule::Train => Term::Train(train(left, right, source)?),
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
