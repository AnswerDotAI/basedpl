//! The binder: the right-to-left reduction of a run's items by their categories, and the rewriting of dot access.

use super::bindings::Address;
use super::tree::{Entity, Fact, Facts, Keyed, Kind, Last, Layer, Level, Mark, Mismatch, Op, Postfix, Read, StatementCode, Split, Target, Term, To, constant, derived, lettered, narrow, paired, piece, postfixed, span_of};
use super::*;

impl Session {
    /// Whether the dot path that starts `nodes`, such as `mul` or `ops.mul`, names a function or operator, read without evaluating anything (`path_category`).
    fn path_function(&self, nodes: &[Node], log: &mut Facts) -> bool {
        let Some(NodeKind::Name(name, _)) = nodes.first().map(|n| &n.kind) else { return false };
        let keys: Vec<Arc<str>> = nodes[1..].chunks_exact(2).map_while(|pair| match pair {
            [Node { kind: NodeKind::Operator(OperatorKind::Dot), .. }, Node { kind: NodeKind::Name(key, _), .. }] => Some(key.text().into()), _ => None
        }).collect();
        self.path_category(*name, &keys, log) != Category::Value
    }
}

/// The groupings that the walk reads (`Compiler::level`): dot reads, joined items and runs that start with an operator. Each reads categories through the compiler that walks the nodes, so the groups match how they compile.
impl Compiler<'_> {
    fn has_members(nodes: &[Node]) -> bool {
        nodes.iter().any(|n| matches!(n.kind, NodeKind::Subscript(_) | NodeKind::Joiner(_))) || nodes.windows(2).any(|w| {
            matches!(w[0].kind, NodeKind::Operator(OperatorKind::Dot))
                && matches!(w[1].kind, NodeKind::Name(_, _) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. })
        })
    }

    /// `nodes` with their dot reads and their joined items grouped, as their categories decide (`category`).
    fn members<'a>(&mut self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        if !Self::has_members(nodes) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        // Whether the last node in `out` is the right operand of a `.` after a function.
        let (mut i, mut on_function) = (0, false);
        while i < nodes.len() {
            if (matches!(nodes[i].kind, NodeKind::Subscript(_)) && on_function
                || matches!(nodes[i].kind, NodeKind::Operator(OperatorKind::Dot)) && matches!(out.last().map(|n| &n.kind), Some(NodeKind::Subscript(_))))
                && let Some(start) = self.operand_start(&out, 0, out.len()) {
                let inner = out.split_off(start);
                let span = crate::syntax::cover(&inner);
                out.push(Node { kind: NodeKind::Group(inner), span });
                on_function = false;
            }
            if let (NodeKind::Operator(OperatorKind::Dot), Some(next)) = (&nodes[i].kind, nodes.get(i + 1)) {
                let root = out.last().filter(|root| {
                    !on_function && match &root.kind {
                        NodeKind::Name(_, _) | NodeKind::Group(_) => self.category(root) == Category::Value,
                        NodeKind::Literal(_) | NodeKind::ArrayLiteral { .. } | NodeKind::Scope(..) => true, _ => false
                    }
                });
                // `.name` reads a field and `.[I]` selects, after an array or a function. After a function, a name that starts a dot path to a function is an inner product's operand instead, as in `+.ops.mul`.
                let field = match (&next.kind, root.is_some()) { (NodeKind::Name(key, _), true) => Some(key.text()), _ => None };
                let read = match &next.kind {
                    NodeKind::Name(name, _) if root.is_some() || !self.session.path_function(&nodes[i + 1..], &mut Some(&mut self.facts)) => {
                        Some((Node { kind: NodeKind::Literal(crate::keyed::text(name.text())), span: next.span.clone() }, Primitive::Mix))
                    }
                    NodeKind::ArrayLiteral { form: ListForm::Items | ListForm::Cells, .. } => Some((next.clone(), Primitive::Bracket)),
                    _ => None,
                };
                if let Some(root) = root {
                    let kind = match (read, &next.kind) {
                        (Some((key, function)), _) => Some(NodeKind::Group(vec![
                            key, Node { kind: NodeKind::Function(Builtin::Primitive(function)), span: nodes[i].span.clone() }, root.clone(),
                        ])),
                        (None, NodeKind::Group(_) | NodeKind::ArrayLiteral { form: ListForm::Rows, .. }) => {
                            Some(NodeKind::Scope(Box::new(root.clone()), Box::new(next.clone())))
                        }
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let root = out.pop().expect("a root was found");
                        let range = root.span.range.start..next.span.range.end;
                        if let Some(key) = field { self.dot_path(&root, key, narrow(&range)); }
                        out.push(Node { kind, span: Span { source: next.span.source.clone(), range } });
                        i += 2;
                        continue;
                    }
                } else if let Some((key, _)) = read {
                    let kind = match &next.kind { NodeKind::Name(name, _) => OperatorKind::Field(*name), _ => OperatorKind::Index };
                    out.push(Node { kind: NodeKind::Operator(kind), span: nodes[i].span.clone() });
                    if matches!(next.kind, NodeKind::ArrayLiteral { .. }) { out.push(key); }
                    (i, on_function) = (i + 2, true);
                    continue;
                }
            }
            on_function = false;
            out.push(nodes[i].clone());
            i += 1;
        }
        // `ⱼ`, `ₑ` or `ᵣ` joins the items it touches once they're bound, as one group (`syntax::joined_term`): `v₁ⱼT.b` joins `1⌷v` and `'b'⊃T`. The left item takes the superscripts it touches, as a literal does, so `x²ⱼy` joins `x²` and `y`. `ₑ` joins first, as in the literal `1ⱼ2ₑ3`, which is `1ⱼ2000`. One beside a function, or apart from an item, stays for `resolve` to reject.
        use crate::syntax::{Joiner, cover, joined_term, touching};
        for exponent in [true, false] {
            let mut i = 1;
            while i + 1 < out.len() {
                let NodeKind::Joiner(joiner) = out[i].kind else { i += 1; continue; };
                let mut start = i - 1;
                while start > 0 && matches!(out[start].kind, NodeKind::BoundOperator(OperatorKind::Power, _) | NodeKind::Operator(OperatorKind::Mark(_) | OperatorKind::Inverse) | NodeKind::Subscript(_) | NodeKind::Letter(_)) && touching(&out[start - 1], &out[start]) {
                    start -= 1;
                }
                let left =
                    if start + 1 == i { out[start].clone() } else { Node { kind: NodeKind::Group(out[start..i].to_vec()), span: cover(&out[start..i]) } };
                if matches!(joiner, Joiner::Exponent) != exponent || !touching(&left, &out[i]) || !touching(&out[i], &out[i + 1])
                    || self.category(&left) != Category::Value || self.category(&out[i + 1]) != Category::Value { i += 1; continue; }
                let joined = joined_term(left, joiner, &out[i].span, out[i + 1].clone());
                out.splice(start..i + 2, [joined]);
                i = start + 1;
            }
        }
        Cow::Owned(out)
    }

    /// Whether a run ends in a dyadic operator, which then takes the next run as its right operand.
    fn ends_in_dyadic_operator(&mut self, nodes: &[Node]) -> bool { self.members(nodes).last().is_some_and(|n| self.category(n).valence() == Some(true)) }

    /// Whether a run starts with an operator, which then takes the whole run before it as its left operand.
    fn starts_with_operator(&mut self, nodes: &[Node]) -> bool { self.members(nodes).first().is_some_and(|n| self.category(n).valence().is_some()) }

    /// Join each run that starts with an operator to the node before it, as the run's first token: `≢⊢ ⌸x` reads as `(≢⊢)⌸x`. A run before it joins as a group. A joined run that still starts with an operator joins again: `+ / ¨x` reads as `+/¨x`. No join reaches across `←`.
    fn joined<'a>(&mut self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        // Every run but the first is tested, so that the facts keep what each test read.
        if !nodes.iter().skip(1).fold(false, |any, node| self.starts(node) || any) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        for node in nodes {
            let mut node = node.clone();
            while out.last().is_some_and(|before| !matches!(before.kind, NodeKind::Assign)) && self.starts(&node) {
                let before = out.pop().expect("a node is before this run");
                let span = Span { source: before.span.source.clone(), range: before.span.range.start..node.span.range.end };
                let before = match before.kind { NodeKind::Run(run) => Node { kind: NodeKind::Group(run), span: before.span }, _ => before };
                let NodeKind::Run(inner) = node.kind else { unreachable!() };
                node = Node { kind: NodeKind::Run(std::iter::once(before).chain(inner).collect()), span };
            }
            out.push(node);
        }
        Cow::Owned(crate::syntax::unwrapped(out))
    }

    /// Whether `node` is a run that starts with an operator, which `joined` joins to the node before it.
    fn starts(&mut self, node: &Node) -> bool { matches!(&node.kind, NodeKind::Run(inner) if self.starts_with_operator(inner)) }

    /// Keeps the dot path of the field read at `range`, which reads `key` from `root`, where `root` is a name or a field read of one, so that `group` binds the read with the field's role (`Session::path_role`).
    fn dot_path(&mut self, root: &Node, key: &str, range: std::ops::Range<u32>) {
        let path = match &root.kind {
            NodeKind::Name(name, _) => Some((*name, Vec::new())),
            _ => { let at = narrow(&root.span.range); self.fields.iter().find(|(r, ..)| *r == at).map(|(_, name, keys)| (*name, keys.clone())) }
        };
        if let Some((name, mut keys)) = path && !self.fields.iter().any(|(at, ..)| *at == range) { keys.push(key.into()); self.fields.push((range, name, keys)); }
    }

    /// Where the operand that ends at `end` in `nodes` starts, no further left than `begin` (`trailing`), compiling nothing.
    fn operand_start(&mut self, nodes: &[Node], begin: usize, end: usize) -> Option<usize> {
        let mark = self.mark();
        let start = self.trailing(nodes, begin, end).ok().map(|(start, _)| start);
        self.truncate(mark);
        start
    }

    /// Fails where `node` is a run whose value is an operator, as `operator` says, unless the run is one parenthesised group. A run reduces to one value: an array, a function, or an operator that a dot path such as `m.op` names.
    fn one_value(&mut self, node: &Node, operator: bool) -> Result<(), Error> {
        let NodeKind::Run(nodes) = &node.kind else { return Ok(()) };
        if operator && !matches!(&self.members(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]) {
            return Err(Error::new(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph").at(&node.span));
        }
        Ok(())
    }

    /// `nodes`, a target of selective assignment, with its dot reads and numeric subscripts made selection calls (`members`), in the group or run that ends it too: `v₁` reads as `1⌷v`.
    fn selection<'a>(&mut self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        let members = self.members(nodes);
        let Some(last) = members.last() else { return members };
        match &last.kind {
            NodeKind::Group(inner) | NodeKind::Run(inner) => {
                let inner = self.selection(inner);
                if matches!(inner, Cow::Borrowed(_)) { return members; }
                let kind = if matches!(last.kind, NodeKind::Run(_)) { NodeKind::Run(inner.into_owned()) } else { NodeKind::Group(inner.into_owned()) };
                let mut out = members.into_owned();
                out.last_mut().unwrap().kind = kind;
                Cow::Owned(out)
            }
            NodeKind::Subscript(index) => {
                let index = Node { kind: NodeKind::Literal(index.clone()), span: last.span.clone() };
                let Some(start) = self.operand_start(&members, 0, members.len() - 1) else { return members };
                let mut out = members.into_owned();
                let subscript = out.pop().unwrap();
                let inner = out.split_off(start);
                let span = crate::syntax::cover(&inner);
                let inner = self.selection(&inner).into_owned();
                let item = if inner.len() == 1 { inner.into_iter().next().unwrap() } else { Node { kind: NodeKind::Group(inner), span } };
                let span = Span { source: item.span.source.clone(), range: item.span.range.start..subscript.span.range.end };
                out.push(Node { kind: NodeKind::Group(vec![
                    index, Node { kind: NodeKind::Function(Builtin::Primitive(Primitive::Index)), span: subscript.span }, item,
                ]), span });
                Cow::Owned(out)
            }
            _ => members,
        }
    }

    /// The target `nodes` as its assignment runs it (`Target`), whose last `count` nodes are a modified assignment's function: the nodes before them, with dot reads and numeric subscripts made selection calls (`selection`), compiled as what they assign to. A plain assignment's target is read by `plain`; a modified one's is names where `assignment_names` admits them and otherwise a selection. Its readings add facts to the statement's, but no assigned names.
    fn assigned_target(&mut self, nodes: &[Node], count: usize) -> Target {
        let assigned = self.assigned.len();
        let selection = self.selection(&nodes[..nodes.len() - count]);
        let mut slots = 0;
        let to = if count == 0 { self.plain(&selection, &mut slots) }
            else if self.session.assignment_names(&selection, &mut Some(&mut self.facts)) { self.names(&selection) }
            else { self.selected(&selection, &mut slots) };
        self.assigned.truncate(assigned);
        Target { to, slots, span: crate::syntax::cover(nodes), at: nodes[0].span.clone() }
    }

    /// What a plain assignment's target `nodes` assigns to: a name, whatever it holds, `⎕`, an unknown system name's error, names (`names`), a function's rule (`Session::rule_target`), or a selection (`selected`). A group holds names or a selection.
    fn plain(&mut self, nodes: &[Node], slots: &mut usize) -> To {
        let session = self.session;
        match nodes {
            [Node { kind: NodeKind::Name(name, _), span }] => To::Name(*name, span.clone()),
            [Node { kind: NodeKind::System(name), span }] => To::Invalid(undefined(name, span)),
            [Node { kind: NodeKind::Output, span }] => To::Output(span.clone()),
            _ if session.assignment_names(nodes, &mut Some(&mut self.facts)) => self.names(nodes),
            [Node { kind: NodeKind::Group(inner), .. }] => self.selected(inner, slots),
            _ => match session.rule_target(nodes, &mut Some(&mut self.facts)) {
                Some((name, derivative)) => To::Rule(name, derivative, nodes[0].span.clone()),
                None => self.selected(nodes, slots),
            },
        }
    }

    /// The names that the target `nodes` assigns, which `assignment_names` admits: a name, or brackets of names whose cells each have the key that a pattern reads (`Session::targets`). A pattern whose cells don't fit gives its error.
    fn names(&mut self, nodes: &[Node]) -> To {
        match nodes {
            [Node { kind: NodeKind::Name(name, _), span }] => To::Name(*name, span.clone()),
            [Node { kind: NodeKind::Group(inner), .. }] => self.names(inner),
            [Node { kind: NodeKind::ArrayLiteral { cells, record, .. }, span }] => match self.session.targets(cells, *record, &mut Some(&mut self.facts)) {
                Ok(cells) => To::Names(cells.into_iter().map(|(key, cell)| (key, self.names(cell), cell[0].span.clone())).collect(), span.clone()),
                Err(e) => To::Invalid(e),
            },
            _ => unreachable!("`assignment_names` admits names, groups and brackets"),
        }
    }

    /// The selection target `nodes`, as layers (`layer`), or the error of one that assigns on its way to its root array.
    fn selected(&mut self, nodes: &[Node], slots: &mut usize) -> To {
        match Session::root_assignment(nodes) {
            Some(span) => To::Invalid(Error::new(ErrorKind::Syntax, INVALID_SELECTION).at(&span)),
            None => To::Selected(self.layer(nodes, slots)),
        }
    }

    /// The layer of a selection target that `nodes` hold, inside the groups and runs that hold them alone (`tree::Layer`). The split is at the first selecting primitive where the nodes before it can all be keys, and the extension of missing keys reaches the level through groups but not through a run (`Split::keyed`); otherwise it's after the first node where that's an array and a selecting primitive and more nodes follow (`Split::simple`). A strand of two arrays, or any last node but a name, a group or a run, can't end a selection.
    fn layer(&mut self, nodes: &[Node], slots: &mut usize) -> Layer {
        let (span, at) = (crate::syntax::cover(nodes), nodes[0].span.clone());
        let (mut nodes, mut keys) = (nodes, true);
        loop {
            match nodes {
                [Node { kind: NodeKind::Group(inner), .. }] => nodes = inner,
                [Node { kind: NodeKind::Run(inner), .. }] => (nodes, keys) = (inner, false),
                _ => break,
            }
        }
        let selects = |node: &Node| matches!(node.kind.primitive(), Some(Primitive::Index | Primitive::Bracket | Primitive::Mix));
        let first = nodes.iter().position(selects);
        let keyed = keys && first.is_some_and(|i| i > 0 && i + 1 < nodes.len() && nodes[..i].iter().all(crate::syntax::key_node));
        let simple = nodes.len() > 2 && selects(&nodes[1]) && self.is_array(&nodes[0]);
        let at_split = if keyed { first } else { simple.then_some(1) };
        let last_at = nodes.len() - 1;
        let split = at_split.map(|i| {
            let index = self.part(&nodes[..i], None);
            let slot = *slots;
            *slots += 1;
            let unit = self.category(&nodes[i + 1]).valence().is_none();
            let container = Box::new(self.layer(&nodes[i + 1..], slots));
            let p = nodes[i].kind.primitive().expect("a selecting primitive");
            Split { p, index, slot, keyed, simple, unit, container, at: nodes[0].span.clone(), p_at: nodes[i].span.clone() }
        });
        let prefix = (last_at > 0 && !split.as_ref().is_some_and(|split| split.simple && split.unit)).then(|| self.part(&nodes[..last_at], Some(Category::Value)));
        let node = &nodes[last_at];
        let last = match &node.kind {
            NodeKind::Name(name, _) => Last::Name(*name, node.span.clone()),
            NodeKind::Group(_) | NodeKind::Run(_) if at_split == Some(last_at - 1) => Last::Container,
            NodeKind::Group(_) | NodeKind::Run(_) => Last::Layer(Box::new(self.layer(std::slice::from_ref(node), slots))),
            _ if last_at > 0 && self.is_array(&nodes[last_at - 1]) && self.is_array(node) => {
                let span = Span { source: node.span.source.clone(), range: nodes[last_at - 1].span.range.start..node.span.range.end };
                Last::Invalid(Error::new(ErrorKind::Syntax, STRAND_TARGET).at(&span))
            }
            _ => Last::Invalid(Error::new(ErrorKind::Syntax, "selection must end in an array name").at(&node.span)),
        };
        Layer { split, prefix, last, last_at: node.span.clone(), span, at }
    }

    /// Whether this compiler reads `node` as an array: a literal, or a node whose category is an array's.
    fn is_array(&mut self, node: &Node) -> bool { matches!(node.kind, NodeKind::Literal(_)) || self.category(node) == Category::Value }

    /// `nodes`, a part of an assignment's target, compiled as a statement of its own, with an item of category `seed` to their right where one is given, for the assignment to run, or the error that compiling it gives, which the assignment raises where it runs the part. Its facts join this statement's, which checks them as it starts.
    fn part(&mut self, nodes: &[Node], seed: Option<Category>) -> Result<StatementCode, Error> {
        let mut compiler = Compiler::new(self.session);
        let seed = seed.map(|category| Item::new(Read::Register(compiler.register()), category, 0..0));
        let result = compiler.level(nodes, 0, seed)?;
        let mut code = compiler.finish(nodes, result);
        for fact in std::mem::take(&mut code.facts) { note(&mut self.facts, fact); }
        Ok(code)
    }
}

// Only the binder has unfinished trains and bound left arguments. Names and groups contain completed values. `Callable` is an array that holds a function, which binds by the item to its right (`Category::before`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category { NoResult, Value, Function, Operator, DyadicOperator, Left, Train, Absent, Callable }

impl Category {
    /// The category of what an item of this category gives once it's built (`Entity::value`): a train or a bound left argument gives a function.
    pub(super) fn settled(self) -> Self { match self { Self::Train | Self::Left => Self::Function, category => category } }
    pub(super) fn of(value: &Binding) -> Self {
        match value {
            Binding::NoResult => Self::NoResult, Binding::Absent => Self::Absent, Binding::Value(_) | Binding::Released => Self::Value,
            Binding::Function(_) => Self::Function, Binding::Operator(op) => Self::of_operator(op)
        }
    }
    pub(super) fn of_operator(op: &OperatorValue) -> Self {
        if matches!(op, OperatorValue::Operator(op) if op.dyadic()) { Self::DyadicOperator } else { Self::Operator }
    }
    /// Whether this is an operator's category, as `Some` of whether the operator is dyadic.
    pub(super) fn valence(self) -> Option<bool> { match self { Self::DyadicOperator => Some(true), Self::Operator => Some(false), _ => None } }
    /// The category of an array: `Callable` where it holds a function as an item, and otherwise `Value`.
    #[inline]
    pub(super) fn array(a: &Value) -> Self { if a.holds_functions() { Self::Callable } else { Self::Value } }
    /// The category that the binder gives `value`. Unlike `of`, it tells an array that holds a function from other arrays (`array`).
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

/// Whether `value` is a postfix operator, which binds an array before its neighbours use it, and stands for a call after one (`tree::Postfix::of`): a digit's or a mark's operator kind, or a bound `⍠`, as a subscript is.
#[inline]
pub(super) fn postfix(value: &Binding) -> bool {
    matches!(value, Binding::Operator(op) if Postfix::of(op).is_some())
}

/// The role that the compiler binds `value` with: the category that the binder gives it (`Category::bound`), and whether it's a postfix operator (`postfix`).
#[inline]
pub(super) fn role(value: &Binding) -> (Category, bool) { (Category::bound(value), postfix(value)) }

/// The role of a name that nothing binds: an array's. A name that is unbound when its statement starts binds as an array, whatever a write later in the statement gives it, so `x+⍎"x←3"` gives 6.
pub(super) const UNBOUND: (Category, bool) = (Category::Value, false);

/// The role (`role`) of what a name holds, `found`, or `UNBOUND` where nothing binds it.
pub(super) fn role_of(found: Option<&Binding>) -> (Category, bool) { found.map_or(UNBOUND, role) }

/// The role (`role`) of the field that `keys` reach from the array that a name's binding `found` holds, or of `found` where `keys` is empty or `found` holds no array. An unbound name, and a field that the array doesn't have, bind as an array. So does a field of an array whose items are numbers or characters at every depth (`numeric`), which isn't read: reading an item of a pending array would compute it, and a trace's input would stop the trace.
pub(super) fn field_role(found: Option<Binding>, keys: &[Arc<str>]) -> (Category, bool) {
    match found {
        Some(Binding::Value(a)) => keys.iter().try_fold(a, |a, key| (!numeric(&a)).then(|| crate::keyed::field(&a, key)).flatten())
            .map_or(UNBOUND, |item| role(&Binding::from_element(item))),
        found => role_of(found.as_ref()),
    }
}

fn numeric(a: &Value) -> bool { a.kind().is_some() }

/// An item on the compiler's stacks: where its instructions read it, its own category and the category it binds with (`Category::before`), its text, whether it's a postfix operator (`postfix`), and whether a call or a derive gave it (`call`). A function that a call gives binds as a value in its statement, but an assignment stores it as the function, and an operator other than a subscript or a superscript takes it as its function operand.
#[derive(Clone, Debug)]
struct Item { read: Read, category: Category, role: Category, range: std::ops::Range<u32>, postfix: bool, call: bool, literal: bool }

impl Item {
    fn new(read: Read, category: Category, range: std::ops::Range<u32>) -> Self { Self { read, category, role: category, range, postfix: false, call: false, literal: false } }
}

/// Pushes `item`, with the category it binds with against the item below it (`Category::before`).
fn push(stack: &mut Vec<Item>, mut item: Item) {
    item.role = item.category.before(stack.last().map(|below| below.role));
    stack.push(item);
}

/// Whether `left` goes onto `stack` now, rather than after the top of `stack` reduces.
fn shifts(left: &Item, stack: &[Item]) -> bool {
    let [.., below, top] = stack else { return true };
    match Rule::get(top.role, below.role) {
        // An operator still awaiting its operand cannot reduce with its right neighbour.
        Rule::Wait(_) => true,
        // Postfix operators bind an array before its neighbours use it as an argument or operand.
        _ if left.postfix => true,
        _ if top.role == Category::Value && below.postfix => false,
        old => match Rule::get(left.category.before(Some(top.role)), top.role) {
            // Adjacent arrays group from the right: `v w i` is `v (w i)`.
            Rule::Adjacent if matches!(old, Rule::Adjacent) => false,
            new => new.strength() >= old.strength(),
        },
    }
}

/// Adds `fact` to `facts` unless they hold it already.
fn note(facts: &mut Vec<Fact>, fact: Fact) { if !facts.contains(&fact) { facts.push(fact) } }

/// The compiler of a statement: the binder's grammar applied to items that instructions compute as the statement runs, so that how a statement binds depends only on its text and on what the names it reads hold when it starts. A call's result is a value to the binder, whatever it holds. It keeps the instructions, constants and levels so far, the registers in use and free, every level's facts about what names hold, which the statement checks as it starts, each name that an assignment earlier in the statement binds, with the category of what it assigned, and the dot path of each field read that `members` formed, by its text (`dot_path`).
pub(super) struct Compiler<'s> {
    session: &'s Session, ops: Vec<Op>, constants: Vec<Entity>, levels: Vec<Option<Level>>, registers: u32, free: Vec<u32>,
    facts: Vec<Fact>, assigned: Vec<(Name, Category)>, fields: Vec<(std::ops::Range<u32>, Name, Vec<Arc<str>>)>,
}

impl<'s> Compiler<'s> {
    /// `nodes` compiled as a statement, with an item of category `seed` to the right of them all where one is given, which `StatementCode::run` takes in register 0.
    pub(super) fn compile(nodes: &[Node], session: &Session, seed: Option<Category>) -> Result<StatementCode, Error> {
        let mut compiler = Compiler::new(session);
        let seed = seed.map(|category| Item::new(Read::Register(compiler.register()), category, 0..0));
        let result = compiler.level(nodes, 0, seed)?;
        Ok(compiler.finish(nodes, result))
    }

    /// The statement `nodes` as this compiler compiled it, whose value `result` holds.
    fn finish(self, nodes: &[Node], result: Item) -> StatementCode {
        let levels = self.levels.into_iter().map(|level| level.expect("a level that the compiler closed")).collect();
        StatementCode { ops: self.ops, constants: self.constants, registers: self.registers as usize, result: result.read, call: result.call, facts: self.facts, span: nodes[0].span.clone(), levels }
    }

    /// A compiler with nothing compiled.
    fn new(session: &'s Session) -> Self {
        Compiler {
            session, ops: Vec::new(), constants: Vec::new(), levels: Vec::new(), registers: 0, free: Vec::new(), facts: Vec::new(), assigned: Vec::new(),
            fields: Vec::new()
        }
    }

    /// Where the operand that ends at `end` starts, no further left than `begin`, with its item, read right to left as `level` reads it. An operand is what the strongest rules join, as an operator joins its operand and a dyadic operator its right operand. An operator waits for its operand, so whatever is to its left joins it until one arrives.
    fn trailing(&mut self, nodes: &[Node], begin: usize, end: usize) -> Result<(usize, Item), Error> {
        if end == begin { return Err(Error::new(ErrorKind::Syntax, "missing assignment operand").at(&nodes[end.min(nodes.len() - 1)].span)); }
        let source = &nodes[0].span.source;
        let strong = |left: Category, right: Category| matches!(Rule::get(left, right), Rule::Derive | Rule::BindRight);
        let (mut stack, mut start) = (Vec::new(), end);
        while start > begin {
            let mark = self.mark();
            let item = self.leaf(&nodes[start - 1], &mut stack)?;
            if let Some(top) = stack.last() && top.category != Category::Operator && !strong(item.category.before(Some(top.role)), top.role) {
                self.truncate(mark);
                break;
            }
            push(&mut stack, item);
            start -= 1;
            while let [.., right, left] = &stack[..] && strong(left.role, right.role) { self.reduce(&mut stack, false, source)?; }
        }
        Ok((start, stack.pop().expect("an operand ends at `end`")))
    }

    /// Compiles `nodes` as level `level`, with `seed` on the stack. Gives the item that holds the level's value.
    fn level(&mut self, nodes: &[Node], level: u32, seed: Option<Item>) -> Result<Item, Error> {
        // Joined runs, dot reads and joined items group the nodes before the walk reads them. The groupings read categories through this compiler, which gives the categories the walk will compile with. Each instruction holds the nodes it reads, so nothing keeps the groups.
        let joined = self.joined(nodes);
        let members = self.members(&joined);
        let nodes = &members[..];
        let source = &nodes[0].span.source;
        let (mut stack, mut pending) = (Vec::new(), Vec::new());
        let (mut cursor, mut assignment) = (nodes.len(), None);
        if let Some(seed) = seed { push(&mut stack, seed); }
        loop {
            let next = if pending.is_empty() && assignment.is_none() && cursor > 0 {
                cursor -= 1;
                if matches!(nodes[cursor].kind, NodeKind::Assign) { assignment = Some(cursor); None } else { Some(self.leaf(&nodes[cursor], &mut stack)?) }
            } else { None };
            if let Some(left) = pending.last() {
                if shifts(left, &stack) {
                    let left = pending.pop().expect("a pending item");
                    push(&mut stack, left);
                    continue;
                }
            } else if let Some(left) = next {
                if shifts(&left, &stack) { push(&mut stack, left); continue; }
                pending.push(left);
            } else if stack.len() < 2 {
                let Some(i) = assignment.take() else { break };
                let (item, start) = self.assignment(nodes, i, stack.pop())?;
                cursor = start;
                pending.push(item);
                continue;
            }
            loop {
                // The statement's last reduction, made on its own level with nothing left to resolve or on either stack and no assignment in progress.
                let last = level == 0 && assignment.is_none() && cursor == 0 && pending.is_empty() && stack.len() == 2;
                self.reduce(&mut stack, last, source)?;
                let m = stack.len();
                // Bound operators still need their left operand before rebinding on the right.
                if matches!(stack[m - 1].category, Category::Operator) || shifts(&stack[m - 1], &stack[..m - 1]) { break; }
                pending.push(stack.pop().expect("a reduced item"));
            }
        }
        Ok(stack.pop().expect("nonempty expression"))
    }

    /// The item for `node`, which builds by a constructor (`tree::constant`) or compiles to instructions: nothing evaluates here. A run that ends in a dyadic operator takes its right operand from the top of `stack`.
    fn leaf(&mut self, node: &Node, stack: &mut Vec<Item>) -> Result<Item, Error> {
        let range = narrow(&node.span.range);
        Ok(match &node.kind {
            NodeKind::Run(inner) if self.ends_in_dyadic_operator(inner) => {
                // A run that ends in a dyadic operator takes the next run as its right operand, because each run acts as one token.
                let Some(right) = stack.pop() else {
                    return Err(Error::new(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it").at(&node.span));
                };
                self.group(node, inner, Some(right))?
            }
            NodeKind::Group(inner) | NodeKind::Run(inner) => self.group(node, inner, None)?,
            NodeKind::ArrayLiteral { .. } => self.literal(node)?,
            NodeKind::Name(name, take) => self.name(*name, *take, range),
            NodeKind::Subscript(index) => self.postfix_item(Postfix::Index(index.clone()), range),
            NodeKind::Letter(letter) => self.postfix_item(lettered(*letter), range),
            NodeKind::BoundOperator(OperatorKind::Power, count) => self.postfix_item(Postfix::Power(count.as_number().expect("a superscript count").integer().expect("an integer superscript")), range),
            NodeKind::Operator(OperatorKind::Inverse) => self.postfix_item(Postfix::Power(-1), range),
            NodeKind::Operator(OperatorKind::Mark(mark)) => self.postfix_item(Postfix::Mark(*mark), range),
            NodeKind::Number { text, mark } => {
                let to = self.register();
                self.ops.push(Op::Number { text: *text, mark: *mark, empty: false, to, range: range.clone() });
                Item { literal: true, ..Item::new(Read::Register(to), Category::Value, range) }
            }
            NodeKind::Zilde(mark) => {
                let to = self.register();
                self.ops.push(Op::Number { text: Some("0"), mark: *mark, empty: true, to, range: range.clone() });
                Item::new(Read::Register(to), Category::Value, range)
            }
            NodeKind::Literal(_) | NodeKind::Function(_) | NodeKind::Operator(_) | NodeKind::BoundOperator(..) => self.constant(Term::Binding(constant(node)), range),
            // A dfn becomes a function or an operator as it's read, which captures the frame that reads it.
            NodeKind::Dfn(definition) => {
                let category = match definition.kind {
                    DefinitionKind::Function => Category::Function, DefinitionKind::MonadicOperator => Category::Operator,
                    DefinitionKind::DyadicOperator => Category::DyadicOperator,
                };
                let to = self.register();
                self.ops.push(Op::Closure { definition: definition.clone(), to, range: range.clone(), check: category });
                Item::new(Read::Register(to), category, range)
            }
            NodeKind::Scope(root, body) => self.scope(root, body, range)?,
            NodeKind::Output => {
                let to = self.register();
                self.ops.push(Op::Input { to, range: range.clone() });
                Item::new(Read::Register(to), Category::Value, range)
            }
            // An unknown system name fails as it's read, where a known one would be a function.
            NodeKind::System(name) => self.fail(undefined(name, &node.span), Category::Function, range),
            NodeKind::Joiner(_) => self.fail(Error::new(ErrorKind::Syntax, "ⱼ, ₑ and ᵣ join the two arrays they touch, as in xⱼy or 2ₑn").at(&node.span), Category::Value, range),
            _ => self.fail(Error::new(ErrorKind::Syntax, "unexpected assignment symbol").at(&node.span), Category::Value, range),
        })
    }

    /// An item for a postfix: a constant that binds as a monadic operator and compiles by its operand's role (`derive`).
    fn postfix_item(&mut self, postfix: Postfix, range: std::ops::Range<u32>) -> Item { Item { postfix: true, ..self.constant(Term::Postfix(postfix), range) } }

    /// An item whose instruction raises `error` where the statement reaches it (`Op::Fail`), binding as `category` until then.
    fn fail(&mut self, error: Error, category: Category, range: std::ops::Range<u32>) -> Item {
        let to = self.register();
        self.ops.push(Op::Fail { error });
        Item::new(Read::Register(to), category, range)
    }

    /// The scope `T.(expr)` or `T.[rows]`: `root` compiled as an item, then the body, which compiles as the statement enters it (`Op::Scope`). What it gives is a value to the binder, as a call's result is.
    fn scope(&mut self, root: &Node, body: &Node, range: std::ops::Range<u32>) -> Result<Item, Error> {
        let root = self.leaf(root, &mut Vec::new())?;
        let body: Arc<[Node]> = match &body.kind { NodeKind::Group(inner) => inner.clone().into(), _ => vec![body.clone()].into() };
        self.release(root.read);
        let to = self.register();
        self.ops.push(Op::Scope { root: root.read, body, to, range: range.clone() });
        Ok(Item::new(Read::Register(to), Category::Value, range))
    }

    /// A read of `name`, which takes the binding out of its frame where `take` marks the dfn's last read of it. A name that an assignment earlier in the statement binds binds as what it assigned. Any other binds with the role of what it holds now (`Session::path_role`), which the statement's facts keep, and its read fails if that role changed while the statement ran (`role_changed`); an unbound name binds as an array. An absent `⍺` drops out, so it compiles to a constant.
    fn name(&mut self, name: Name, take: bool, range: std::ops::Range<u32>) -> Item {
        let session = self.session;
        let (category, postfix, on) = match self.assigned.iter().rev().find(|(assigned, _)| *assigned == name) {
            Some(&(_, category)) => (category, false, Mismatch::Fit),
            None => {
                let (category, postfix) = session.path_role(name, &[], &mut Some(&mut self.facts));
                if category == Category::Absent { return self.constant(Term::Binding(Binding::Absent), range); }
                (category, postfix, Mismatch::Fail)
            }
        };
        let to = self.register();
        self.ops.push(Op::Load { name, address: session.bindings.address(name), take, to, range: range.clone(), check: category, on });
        Item { postfix, ..Item::new(Read::Register(to), category, range) }
    }

    /// The item for the group or run `node`, whose nodes are `inner`, compiled as a level of its own, with `seed` on its stack where it's a run that ends in a dyadic operator. A group with a constant value compiles to that constant. A field read that `members` formed binds with the field's role (`dot_path`, `Session::path_role`), which the statement's facts keep. Any other group binds as its value does, where a train or a bound left argument becomes a function.
    fn group(&mut self, node: &Node, inner: &[Node], seed: Option<Item>) -> Result<Item, Error> {
        let session = self.session;
        let range = narrow(&node.span.range);
        let start = self.mark();
        self.levels.push(None);
        let id = self.levels.len() as u32;
        self.ops.push(Op::Enter { level: id });
        let value = session.nested(inner, |_| self.level(inner, id, seed))?;
        self.one_value(node, value.category.valence().is_some())?;
        let path = self.fields.iter().find(|(at, ..)| *at == range).map(|(_, name, keys)| (*name, keys.clone()));
        let (category, postfix) = match path {
            Some((name, keys)) => session.path_role(name, &keys, &mut Some(&mut self.facts)),
            None => (value.category.settled(), value.postfix),
        };
        if let Read::Constant(c) = value.read && self.ops.len() == start.ops + 1 {
            let constant = self.constants[c as usize].clone().value(&node.span.source)?;
            self.truncate(start);
            return Ok(Item { postfix, ..self.constant(Term::Binding(constant), range) });
        }
        self.ops.push(Op::Exit { level: id, from: value.read });
        self.release(value.read);
        let to = self.register();
        self.levels[id as usize - 1] = Some(Level { to, range: range.clone(), span: inner[0].span.clone(), check: category, kind: Kind::Group });
        Ok(Item { postfix, call: value.call, literal: value.literal, ..Item::new(Read::Register(to), category, range) })
    }

    /// The item for the bracket literal `node`. Its items, and its keys that aren't constant, are levels of their own (`Compiler::item`), which run left to right with an item's value before its key, and the literal's instruction assembles what they give (`tree::assembled`). The level that finishes a keyed item makes its entries before the next item runs (`tree::Keyed`). An absent `⍺` drops out, and so does a keyed item whose value is absent, without its key being evaluated. A key with no value is a name, which gives both: `[a:]` is `["a":a]`. A literal whose pieces are all constant compiles to its value. A literal binds as an array that holds a function where an item binds as a function.
    fn literal(&mut self, node: &Node) -> Result<Item, Error> {
        let NodeKind::ArrayLiteral { cells, record, .. } = &node.kind else { unreachable!("a bracket literal") };
        let source = &node.span.source;
        let range = narrow(&node.span.range);
        let (start, shared) = (self.mark(), std::cell::OnceCell::new());
        let (mut pieces, mut functions) = (Vec::with_capacity(cells.len()), false);
        for (k, nodes) in cells.iter().enumerate() {
            let Some(colon) = record.then(|| crate::syntax::key_colon(nodes)).flatten() else {
                let (item, _) = self.item(node, &shared, k, 0..nodes.len())?;
                functions |= item.category.settled() == Category::Function;
                if item.category != Category::Absent { pieces.push(item.read) }
                continue;
            };
            // The value and the key are items of their own, value first. As one expression, `key:f` would be a train.
            let named = match &nodes[..colon] {
                _ if colon + 1 < nodes.len() => None,
                [Node { kind: NodeKind::Name(name, _), .. }] => Some(crate::keyed::text(name.text())),
                _ => return Err(Error::new(ErrorKind::Syntax, "a key with no value is a name, as in [a:]").at(&nodes[colon].span)),
            };
            let at = if named.is_some() { 0..colon } else { colon + 1..nodes.len() };
            let text = narrow(&crate::syntax::cover(&nodes[at.clone()]).range);
            let (value, held) = self.item(node, &shared, k, at)?;
            functions |= value.category.settled() == Category::Function;
            // An absent value leaves its item out, key and all, and its key isn't evaluated.
            if value.category == Category::Absent { self.release(value.read); continue }
            let key = match named {
                Some(key) => key,
                None => match self.item(node, &shared, k, 0..colon)? {
                    (Item { read: Read::Constant(c), .. }, _) => {
                        self.constants[c as usize].clone().value(source)?.into_value(&crate::syntax::cover(&nodes[..colon]))?
                    }
                    // A key that isn't constant makes the item's entries as its own level ends, from the value that evaluated before it.
                    (key, Some(keyed)) => {
                        let Some(Level { kind: Kind::Item { key: slot, .. }, .. }) = &mut self.levels[keyed as usize - 1] else { unreachable!("an item's level") };
                        *slot = Some(Keyed::Value { value: value.read, colon: colon as u32 });
                        self.release(value.read);
                        pieces.push(key.read);
                        continue;
                    }
                    (_, None) => unreachable!("a computed item has a level"),
                },
            };
            match (value.read, held) {
                (Read::Constant(c), _) => {
                    let value = self.constants[c as usize].clone().value(source)?;
                    pieces.push(self.constant(piece(self.session, node, k, value, Some((&key, colon)), &node.span)?, text).read);
                }
                (read, Some(held)) => {
                    let Read::Constant(constant) = self.constant(Term::Binding(Binding::Value(key)), text).read else { unreachable!("a constant") };
                    let Some(Level { kind: Kind::Item { key: slot, .. }, .. }) = &mut self.levels[held as usize - 1] else { unreachable!("an item's level") };
                    *slot = Some(Keyed::Key { constant, colon: colon as u32 });
                    pieces.push(read);
                }
                (Read::Register(_), None) => unreachable!("a computed item has a level"),
            }
        }
        let category = if functions { Category::Callable } else { Category::Value };
        let constants = &self.constants;
        let folded = if matches!(node.kind, NodeKind::ArrayLiteral { form: ListForm::Cells, .. }) { None } else {
            pieces.iter().map(|piece| match piece { Read::Constant(c) => Some(Cow::Borrowed(&constants[*c as usize])), Read::Register(_) => None })
                .collect::<Option<Vec<_>>>().and_then(|pieces| super::tree::assembled(node, pieces.into_iter()).ok())
        };
        if let Some(value) = folded {
            self.truncate(start);
            return Ok(self.constant(Term::Binding(value), range));
        }
        for &piece in &pieces { self.release(piece) }
        let to = self.register();
        self.ops.push(Op::Array { literal: shared.get_or_init(|| Arc::new(node.clone())).clone(), pieces: pieces.into_boxed_slice(), to, range: range.clone(), check: category });
        Ok(Item::new(Read::Register(to), category, range))
    }

    /// The item for the nodes in `at` of cell `cell` of the bracket literal `node`, compiled as a level of its own, with that level where the item isn't constant. A constant item compiles to its value as the literal takes it (`session::listed`), or to an absent `⍺`, which drops out. The item keeps its own category, which says whether the literal holds a function. `shared` holds the literal for the instruction and the levels that need it, copied once.
    fn item(&mut self, node: &Node, shared: &std::cell::OnceCell<Arc<Node>>, cell: usize, at: std::ops::Range<usize>) -> Result<(Item, Option<u32>), Error> {
        let NodeKind::ArrayLiteral { cells, .. } = &node.kind else { unreachable!("a bracket literal") };
        let session = self.session;
        let nodes = &cells[cell][at.clone()];
        let span = crate::syntax::cover(nodes);
        let text = narrow(&span.range);
        let start = self.mark();
        self.levels.push(None);
        let id = self.levels.len() as u32;
        self.ops.push(Op::Enter { level: id });
        let value = session.nested(nodes, |_| self.level(nodes, id, None))?;
        if !matches!(node.kind, NodeKind::ArrayLiteral { record: true, .. }) && let Read::Constant(c) = value.read && self.ops.len() == start.ops + 1 {
            let constant = self.constants[c as usize].clone().value(&span.source)?;
            self.truncate(start);
            let item = self.constant(piece(session, node, cell, constant, None, &span)?, text);
            return Ok((Item { category: value.category, ..item }, None));
        }
        self.ops.push(Op::ExitItem { level: id, from: value.read });
        self.release(value.read);
        let to = self.register();
        self.levels[id as usize - 1] = Some(Level {
            to, range: text.clone(), span: nodes[0].span.clone(), check: Category::Value,
            kind: Kind::Item { literal: shared.get_or_init(|| Arc::new(node.clone())).clone(), cell: cell as u32, key: None }
        });
        Ok((Item::new(Read::Register(to), value.category, text), Some(id)))
    }

    /// The assignment whose `←` is at `i` among `nodes`, of `value`, or with no value, `a(f)←`'s update. Gives the item that holds what the assignment gives, which is shy, and the position where its target starts, where the walk carries on. A target that is one name gives that name the category of `value` for the rest of the statement.
    fn assignment(&mut self, nodes: &[Node], i: usize, value: Option<Item>) -> Result<(Item, usize), Error> {
        let session = self.session;
        let span = &nodes[i].span;
        let begin = nodes[..i].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |j| j + 1);
        if begin == i { return Err(Error::new(ErrorKind::Syntax, NO_TARGET).at(span)); }
        match value.as_ref().map(|value| value.category) {
            Some(Category::NoResult) => return Err(Error::new(ErrorKind::Value, "assignment requires a value").at(span)),
            Some(Category::Absent) => return Err(Error::new(ErrorKind::Value, ABSENT).at(span)),
            _ => {}
        }
        // A one-node target starts where it is, whatever its name holds, so only a longer one reads the roles of what its names hold.
        let (start, modifier) = if i - begin == 1 { (begin, None) } else { self.target(nodes, begin, i)? };
        let target = self.assigned_target(&nodes[start..i], modifier.as_ref().map_or(0, |(count, _)| *count as usize));
        let category = value.as_ref().map_or(Category::NoResult, |value| value.category.settled());
        let local = match (&nodes[start..i], &value) {
            ([Node { kind: NodeKind::Name(name, _), .. }], Some(_)) => {
                self.assigned.push((*name, category));
                match session.bindings.here(*name) { Place::Slot(_, slot) => Some(slot), _ => None }
            }
            _ => None,
        };
        let call = value.as_ref().is_some_and(|value| value.call);
        let value = value.map(|value| { self.release(value.read); value.read });
        let modifier = modifier.map(|(_, item): (u32, Item)| { self.release(item.read); item.read });
        let to = self.register();
        let range = narrow(&span.range);
        self.ops.push(Op::Assign { value, local, target, span: span.clone(), modifier, to, range: range.clone(), call });
        Ok((Item { call, ..Item::new(Read::Register(to), category, range) }, start))
    }

    /// Where the target of the assignment whose `←` is at `i` starts, no further left than `begin`, with a modified assignment's function: its count of nodes, and its item, which compiles here as any function does. The target is the operand before `←` (`trailing`). Where that operand is a function with an array before it, the function modifies, and the target is the operand before the function. In a dfn a name before `←` is the target unless an array comes directly before it. Then the name's value decides, as at top level, and a function makes a modified assignment, as in `a(f)←3` and `(a)f←3`.
    fn target(&mut self, nodes: &[Node], begin: usize, i: usize) -> Result<(usize, Option<(u32, Item)>), Error> {
        let session = self.session;
        if session.rule_target(&nodes[begin..i], &mut Some(&mut self.facts)).is_some() { return Ok((begin, None)); }
        let mark = self.mark();
        let (operand, item) = self.trailing(nodes, begin, i)?;
        if item.category.valence().is_some() {
            return Err(Error::new(ErrorKind::Syntax, "missing assignment operand").at(&span_of(&nodes[0].span.source, &item.range)));
        }
        let modified = operand > begin && item.category.before(None).settled() == Category::Function
            && (self.category(&nodes[operand - 1]) == Category::Value || !session.assignment_names(&nodes[operand..i], &mut Some(&mut self.facts)));
        let start = if modified {
            let read = self.mark();
            let start = self.trailing(nodes, begin, operand).map(|(start, _)| start);
            self.truncate(read);
            start?
        } else {
            self.truncate(mark);
            operand
        };
        if start > begin && self.category(&nodes[start]) == Category::Value && self.category(&nodes[start - 1]) == Category::Value {
            let span = Span { source: nodes[start].span.source.clone(), range: nodes[start - 1].span.range.start..nodes[start].span.range.end };
            return Err(Error::new(ErrorKind::Syntax, STRAND_TARGET).at(&span));
        }
        Ok((start, modified.then(|| ((i - operand) as u32, item))))
    }

    /// The category that this compiler gives `node` read alone, compiling nothing: a group's or a run's is its value's, a dot path's is its field's, and a run that ends in a dyadic operator is a dyadic operator, as `leaf` reads it. An absent `⍺` counts as an array, as an unbound name does, and a node that can't compile has no result.
    fn category(&mut self, node: &Node) -> Category {
        if let NodeKind::Run(inner) = &node.kind && self.ends_in_dyadic_operator(inner) { return Category::DyadicOperator; }
        let mark = self.mark();
        let read = self.leaf(node, &mut Vec::new()).map(|item| item.category.before(None).settled());
        self.truncate(mark);
        match read { Ok(Category::Absent) => Category::Value, Ok(category) => category, Err(_) => Category::NoResult }
    }

    /// Compiles the reduction of the top two items of `stack`, by the rule that their roles give. `last` marks the statement's last reduction on its own level.
    fn reduce(&mut self, stack: &mut Vec<Item>, last: bool, source: &Arc<Source>) -> Result<(), Error> {
        let (left, right) = (stack.pop().expect("a left item"), stack.pop().expect("a right item"));
        let span = span_of(source, &left.range);
        let item = match Rule::get(left.role, right.role) {
            Rule::Missing => return Err(Error::new(ErrorKind::Value, "expression produced no value").at(&span)),
            Rule::Absent => return Err(Error::new(ErrorKind::Value, ABSENT).at(&span)),
            Rule::Drop => { self.release(left.read); right }
            Rule::DropRight => { self.release(right.read); left }
            // Arrays side by side form a strand, one item each, as in `[a b c]`.
            Rule::Adjacent => {
                let range = left.range.clone();
                self.folded(left, right, Category::Value, range, source, |left, right, to| Op::Strand { left, right, to })
            }
            Rule::Derive => self.derive(left, right, span, source),
            Rule::BindRight => {
                let (range, call) = (left.range.clone(), right.call);
                let literal = right.literal;
                self.folded(left, right, Category::Operator, range, source, |operator, operand, to| Op::BindRight { operator, operand, to, call, literal })
            }
            // A call belongs to its function's text, so a bound left argument takes `right`'s.
            Rule::Attach => {
                let range = right.range.clone();
                let literal = left.literal;
                self.folded(left, right, Category::Left, range, source, |left, right, to| Op::Attach { left, right, to, literal })
            }
            Rule::Call if right.category == Category::Absent => {
                self.release(left.read);
                self.constant(Term::Binding(Binding::Absent), left.range)
            }
            Rule::Call => self.call(left, right, span, last),
            Rule::Train => {
                let range = left.range.clone();
                let literals = [left.literal, right.literal];
                self.folded(left, right, Category::Train, range, source, |left, right, to| Op::Train { left, right, to, literals })
            }
            _ => return Err(Error::new(ErrorKind::Syntax, "these grammatical categories do not bind").at(&span)),
        };
        push(stack, item);
        Ok(())
    }

    /// `left` and `right` reduced by the instruction that `op` makes, or, where both are constants, by running that instruction's step on them (`tree::paired`), which gives a constant where it succeeds; where it fails, the instruction fails as the statement runs. The item binds as `category`, with the text `range`.
    fn folded(&mut self, left: Item, right: Item, category: Category, range: std::ops::Range<u32>, source: &Arc<Source>, op: impl Fn(Read, Read, u32) -> Op) -> Item {
        if let (Read::Constant(l), Read::Constant(r)) = (left.read, right.read)
            && let Ok(entity) = paired(&op(left.read, right.read, 0), Cow::Borrowed(&self.constants[l as usize]), Cow::Borrowed(&self.constants[r as usize]), source, false) {
            return self.constant(entity.term, range);
        }
        self.release(left.read);
        self.release(right.read);
        let to = self.register();
        self.ops.push(op(left.read, right.read, to));
        Item::new(Read::Register(to), category, range)
    }

    /// `operator` applied to `operand` at `span`. A postfix compiles by its operand's role (`postfix`). Otherwise a constant, where both are constants and the derive gives a binding (`derived`), or an instruction. What it gives binds as `derived_category` says, with the operand's text.
    fn derive(&mut self, operand: Item, operator: Item, span: Span, source: &Arc<Source>) -> Item {
        if let Read::Constant(p) = operator.read && let Term::Postfix(postfix) = &self.constants[p as usize].term {
            let postfix = postfix.clone();
            return self.postfix(operand, operator, postfix, span, source);
        }
        let category = self.derived_category(&operand, &operator);
        let range = operand.range.clone();
        if let (Read::Constant(o), Read::Constant(p)) = (operand.read, operator.read) {
            let o = self.constants[o as usize].clone().value(source);
            if let (Ok(o), Ok(p)) = (o, self.constants[p as usize].clone().value(source)) && let Ok(binding) = derived(o, p, operand.literal, &span) {
                return self.constant(Term::Binding(binding), range);
            }
        }
        self.release(operand.read);
        self.release(operator.read);
        let to = self.register();
        self.ops.push(Op::Derive {
            operand: operand.read, operator: operator.read, to, span, range: range.clone(), result: category,
            call: operand.call && !operator.postfix, literal: operand.literal
        });
        Item { call: true, ..Item::new(Read::Register(to), category, range) }
    }

    /// The category that what `operator` gives on `operand` binds as: an array for the call that a postfix operator that a name holds stands for after an array (`tree::Postfix::of`), and a function otherwise.
    fn derived_category(&self, operand: &Item, operator: &Item) -> Category {
        if operator.postfix && operand.role == Category::Value { Category::Value } else { Category::Function }
    }

    /// The postfix `postfix`, whose item is `operator`, compiled for `operand`'s role (target parts 18, 21): after an array, the call that it stands for; after a function, the operator that it stands for, derived as any other (`Postfix::operator`), or `ᵘ`'s scalar. A constant operand folds once (`tree::postfixed`); a postfix that the operand's role rejects compiles to its error (`Op::Fail`).
    fn postfix(&mut self, operand: Item, operator: Item, postfix: Postfix, span: Span, source: &Arc<Source>) -> Item {
        let range = operand.range.clone();
        let array = operand.role == Category::Value;
        if !array && !matches!(postfix, Postfix::Unit) {
            return match postfix.operator(&span) {
                Ok(op) => {
                    let op = Item { postfix: true, ..self.constant(Term::Binding(Binding::Operator(op)), operator.range) };
                    self.derive(operand, op, span, source)
                }
                Err(error) => { self.release(operand.read); self.fail(error, Category::Function, range) }
            };
        }
        if array && matches!(postfix, Postfix::Unit) {
            self.release(operand.read);
            return self.fail(Error::domain("ᵘ makes a scalar that holds a function. Enclose a subject with ⊂").at(&span), Category::Value, range);
        }
        let category = if array { Category::Value } else { Category::Callable };
        if !array && let Read::Constant(o) = operand.read && let Ok(o) = self.constants[o as usize].clone().value(source) && let Ok(binding) = postfixed(&postfix, o, &span) {
            return self.constant(Term::Binding(binding), range);
        }
        self.release(operand.read);
        let to = self.register();
        self.ops.push(Op::Postfix { postfix, operand: operand.read, to, span, range: range.clone(), result: category });
        Item { call: true, ..Item::new(Read::Register(to), category, range) }
    }

    /// The call of `function` on `argument` at `span`, compiled as one instruction with the left argument that the attach before it bound, where it did. A constant train builds its function here, once (`built`); a train whose tines aren't all constants builds its value at each evaluation and is called as any function is. The call's result is a value to the binder, whatever it holds, so that how a statement binds depends only on its text and on the roles of the names it reads, not on what a call returns. `last` marks the statement's last reduction on its own level, which returns a call of a dfn or a derived function to the caller's loop.
    fn call(&mut self, function: Item, argument: Item, span: Span, last: bool) -> Item {
        let range = narrow(&span.range);
        let read = function.read;
        let (left, function, left_literal) = match self.ops.last() {
            Some(&Op::Attach { left, right, to, literal }) if read == Read::Register(to) => { self.ops.pop(); (Some(left), right, literal) }
            _ => (None, read, false),
        };
        let function = self.built(function, &span);
        let literals = u64::from(left_literal) | (u64::from(argument.literal) << 1);
        self.release(read);
        self.release(argument.read);
        let (argument, to) = (argument.read, self.register());
        self.ops.push(Op::Call { left, function, argument, to, span, range: range.clone(), result: Category::Value, last, literals });
        Item { call: true, ..Item::new(Read::Register(to), Category::Value, range) }
    }

    /// `read` as a called function: a constant train, all of whose tines are constants, builds its function here, once (target part 19: 1 `Train` value); any other read stays as it is.
    fn built(&mut self, read: Read, span: &Span) -> Read {
        let Read::Constant(c) = read else { return read };
        if !matches!(self.constants[c as usize].term, Term::Train(_)) { return read; }
        let range = self.constants[c as usize].range.clone();
        match self.constants[c as usize].clone().value(&span.source) { Ok(function) => self.constant(Term::Binding(function), range).read, Err(_) => read }
    }

    /// An item that reads `term`, kept as a constant with the text `range`, which binds as the term's category.
    fn constant(&mut self, term: Term, range: std::ops::Range<u32>) -> Item {
        let literal = matches!(&term, Term::Binding(Binding::Value(Value::Number(_) | Value::Character(_))));
        let entity = Entity::new(term, range.clone(), false, None);
        let category = entity.category();
        self.constants.push(entity);
        Item { literal, ..Item::new(Read::Constant(self.constants.len() as u32 - 1), category, range) }
    }

    /// A register that holds nothing the compiler still needs.
    fn register(&mut self) -> u32 {
        let r = self.free.pop().unwrap_or_else(|| { self.registers += 1; self.registers - 1 });
        r
    }

    /// Frees `read`'s register, whose item a reduction used up or dropped.
    fn release(&mut self, read: Read) { if let Read::Register(r) = read { self.free.push(r) } }

    fn mark(&self) -> Mark {
        Mark { ops: self.ops.len(), constants: self.constants.len(), levels: self.levels.len(), registers: self.registers, free: self.free.clone() }
    }

    /// Drops the instructions, constants, levels and registers that the compiler gained after `mark`: a constant group's, or those of a literal that compiles to its value.
    fn truncate(&mut self, mark: Mark) {
        self.ops.truncate(mark.ops);
        self.constants.truncate(mark.constants);
        self.levels.truncate(mark.levels);
        (self.registers, self.free) = (mark.registers, mark.free);
    }
}
