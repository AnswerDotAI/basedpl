//! Recorded statements, step 9 of `meta/programs.md` ("Cached statement trees"). The binder logs the steps it takes on a dfn statement's first evaluation (`Recorder`): each node it resolves, and whether it shifts the node onto its stack or holds it on its pending stack, each move between those stacks, and each reduction. Later evaluations replay the log (`Recording::replay`), which makes the same calls in the same order with no category decisions, dot-access scans or node dispatch. The binder's decisions depend only on categories, so the replay checks the category of each name, group and call result against the recorded one. Where one differs, the binder carries on from the replay's state (`Binder::resumed`).
use super::*;
use super::binder::{attached, narrow, split_call, strand, tail_call, Binder, Category, Entity, Resume, Stacked, Term};
use std::{borrow::Cow, ops::Range, sync::OnceLock};

/// The most items that a replay keeps on either stack. A statement that needs more isn't recorded.
const DEPTH: usize = 8;

/// A statement's recorded binder steps, with the runs whose operator tests depend on what names hold.
#[derive(Clone, Debug)]
pub(crate) struct Recording { ops: Vec<Recorded>, guards: Vec<Guard> }

/// A run, by node, that a name could turn into one the binder treats differently: one whose first member is an operator, which joins the run to the node before it (`Starts`), or one whose last member is a dyadic operator, which takes the next run as its operand (`Ends`).
#[derive(Clone, Debug)]
enum Guard { Starts(usize), Ends(usize) }

/// One binder step.
#[derive(Clone, Debug)]
pub(super) enum Recorded {
    /// The next node, resolved, then shifted onto the stack or held on the pending stack. `range` is the node's text.
    Next { leaf: Leaf, shift: bool, range: Range<u32> },
    /// The top of the pending stack shifts onto the stack.
    Unpend,
    /// The top of the stack goes back to the pending stack.
    Pend,
    /// The top two items of the stack reduce.
    Reduce(Reduction),
}

/// A resolved node. A category is the one that the node had when recorded.
#[derive(Clone, Debug)]
pub(super) enum Leaf {
    Value(Value),
    Function(Function),
    /// Any other node that `Session::resolve` evaluates.
    Resolve(Category),
    /// A group or run, with its own recording.
    Group(Recording, Category),
}

#[derive(Clone, Debug)]
pub(super) enum Reduction {
    /// An absent `⍺` drops out.
    Drop,
    /// An absent item on the right drops out.
    DropRight,
    Strand,
    Attach,
    /// A call, made at the function's text `span`, whose recorded result had category `result`. `range` is the result's text.
    Call { span: Span, range: Range<u32>, result: Category },
    /// An attach and then its call: a dyadic call, made as `Call` is.
    Dyadic { span: Span, range: Range<u32>, result: Category },
}

/// The steps logged while the binder evaluates a statement. `held` is the leaf just resolved, until the binder shifts or holds it, and `call` marks a call that `reduce` has returned and not yet made. `failed` records a step outside the subset that recordings cover, and `deepest` is the most items either of the binder's stacks has held.
#[derive(Default)]
pub(super) struct Recorder { ops: Vec<Recorded>, held: Option<(Leaf, Range<u32>)>, pub(super) call: bool, failed: bool, deepest: usize }

impl Recorder {
    pub(super) fn fail(&mut self) { self.failed = true; }
    pub(super) fn push(&mut self, op: Recorded) { self.ops.push(op); }
    /// Notes the lengths of the binder's two stacks, which it reports before taking each node.
    pub(super) fn depth(&mut self, stack: usize, pending: usize) { self.deepest = self.deepest.max(stack).max(pending); }
    /// Holds the leaf for `node`, which resolved to `binding`.
    pub(super) fn leaf(&mut self, node: &Node, binding: &Binding) {
        self.held = Some((match (&node.kind, binding) {
            (NodeKind::Literal(_), Binding::Value(v)) => Leaf::Value(v.clone()),
            (NodeKind::Function(_), Binding::Function(f)) => Leaf::Function(f.clone()),
            _ => Leaf::Resolve(Category::of(binding)),
        }, narrow(&node.span.range)));
    }
    /// Holds the leaf for the group or run `node`, which `child` recorded and which gave `bound`.
    pub(super) fn group(&mut self, node: &Node, child: Recorder, bound: &Bound) {
        let (NodeKind::Group(inner) | NodeKind::Run(inner)) = &node.kind else { unreachable!() };
        match child.finish(inner) { Some(recording) => self.held = Some((Leaf::Group(recording, Category::of(&bound.value)), narrow(&node.span.range))), None => self.failed = true }
    }
    /// Logs the held leaf, which the binder shifted, or held when `shift` is false.
    pub(super) fn next(&mut self, shift: bool) {
        match self.held.take() { Some((leaf, range)) => self.ops.push(Recorded::Next { leaf, shift, range }), None => self.failed = true }
    }
    /// Logs the call that `reduce` returned, made at `span`, whose result had category `result`. A call straight after the attach that gives its left argument logs as one dyadic call.
    pub(super) fn called(&mut self, span: Span, result: Category) {
        if !std::mem::take(&mut self.call) { self.failed = true; return; }
        let range = narrow(&span.range);
        let dyadic = matches!(self.ops.last(), Some(Recorded::Reduce(Reduction::Attach)));
        if dyadic { self.ops.pop(); }
        self.ops.push(Recorded::Reduce(if dyadic { Reduction::Dyadic { span, range, result } } else { Reduction::Call { span, range, result } }));
    }
    /// The recording of `nodes`, unless a step was outside the subset or a stack held more than `DEPTH` items.
    pub(super) fn finish(self, nodes: &[Node]) -> Option<Recording> {
        if self.failed || self.held.is_some() || self.call || self.deepest > DEPTH { return None; }
        let named = |node: Option<&Node>| node.is_some_and(Session::names_valence);
        let mut guards = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            let NodeKind::Run(inner) = &node.kind else { continue };
            if i > 0 && named(inner.first()) { guards.push(Guard::Starts(i)); }
            if named(inner.last()) { guards.push(Guard::Ends(i)); }
        }
        Some(Recording { ops: self.ops, guards })
    }
}

/// An item on a replay's stacks: an entity's term, with literals and primitives borrowed from the recording.
enum Part<'p> { Array(Cow<'p, Value>), Function(Cow<'p, Function>), Other(Binding), Left(Cow<'p, Value>, Cow<'p, Function>), Strand(Vec<Value>) }
/// An entity on a replay's stacks: its term, its text and whether it displays.
struct Item<'p> { part: Part<'p>, range: Range<u32>, shy: bool }

impl<'p> Item<'p> {
    fn of(binding: Binding, range: Range<u32>, shy: bool) -> Self {
        let part = match binding { Binding::Value(v) => Part::Array(Cow::Owned(v)), Binding::Function(f) => Part::Function(Cow::Owned(f)), other => Part::Other(other) };
        Self { part, range, shy }
    }
    fn category(&self) -> Category {
        match &self.part { Part::Array(_) | Part::Strand(_) => Category::Value, Part::Function(_) => Category::Function, Part::Left(..) => Category::Left, Part::Other(b) => Category::of(b) }
    }
    /// The binder's entity for this item.
    fn entity(self) -> Entity {
        let term = match self.part {
            Part::Array(v) => Term::Binding(Binding::Value(v.into_owned())),
            Part::Function(f) => Term::Binding(Binding::Function(f.into_owned())),
            Part::Other(b) => Term::Binding(b),
            Part::Left(x, f) => Term::Left(x.into_owned(), f.into_owned()),
            Part::Strand(items) => Term::Strand(items),
        };
        Entity { term, range: self.range, shy: self.shy, expression: None }
    }
    /// The item's value, as `Entity::value` gives it.
    fn binding(self, source: &Arc<Source>) -> Result<Binding, Error> {
        match self.part {
            Part::Array(v) => Ok(Binding::Value(v.into_owned())),
            Part::Function(f) => Ok(Binding::Function(f.into_owned())),
            part => Item { part, ..self }.entity().value(source),
        }
    }
}

/// Literals and primitives stay borrowed. Every other part converts as the binder's `Entity` does.
impl<'p> Stacked for Item<'p> {
    type Array = Cow<'p, Value>;
    type Function = Cow<'p, Function>;
    fn range(&self) -> Range<u32> { self.range.clone() }
    fn argument(self, source: &Arc<Source>) -> Result<Cow<'p, Value>, Error> {
        match self.part { Part::Array(v) => Ok(v), part => Ok(Cow::Owned(Item { part, ..self }.entity().array(source)?)) }
    }
    fn called(self, source: &Arc<Source>) -> Result<Cow<'p, Function>, Error> {
        match self.part { Part::Function(f) => Ok(f), part => Ok(Cow::Owned(Item { part, ..self }.entity().function(source)?)) }
    }
    fn into_strand(self) -> Result<Vec<Value>, Self> { match self.part { Part::Strand(items) => Ok(items), part => Err(Item { part, ..self }) } }
    fn into_left(self) -> Result<(Cow<'p, Value>, Cow<'p, Function>), Self> { match self.part { Part::Left(x, f) => Ok((x, f)), part => Err(Item { part, ..self }) } }
    fn owned(array: Cow<'p, Value>) -> Value { array.into_owned() }
}

/// One of the binder's stacks, held inline: a recording's depth is known.
struct Stack<'p> { items: [Option<Item<'p>>; DEPTH], len: usize }

impl<'p> Stack<'p> {
    fn new() -> Self { Self { items: std::array::from_fn(|_| None), len: 0 } }
    fn push(&mut self, item: Item<'p>) { self.items[self.len] = Some(item); self.len += 1; }
    fn pop(&mut self) -> Item<'p> { self.len -= 1; self.items[self.len].take().expect("a replay pops only what it pushed") }
    fn entities(self) -> Vec<Entity> { self.items.into_iter().flatten().map(Item::entity).collect() }
}

/// The binder, carrying on from a replay's state: its stacks and cursor, with the entity just resolved, or after a call's result.
#[allow(clippy::too_many_arguments)]
fn resumed(nodes: &[Node], session: &mut Session, tail: bool, stack: Stack<'_>, pending: Stack<'_>, cursor: usize, next: Option<Item<'_>>, after_call: bool) -> Result<Step, Error> {
    Binder::resumed(nodes, session, tail, Resume { stack: stack.entities(), pending: pending.entities(), cursor, next: next.map(Item::entity), after_call })
}

/// The run at node `i`'s members.
fn run(nodes: &[Node], i: usize) -> &[Node] { let NodeKind::Run(inner) = &nodes[i].kind else { unreachable!("a guard names a run") }; inner }

impl Recording {
    /// `nodes` evaluated by replaying the recorded steps, which gives what `Binder::evaluate` gives.
    fn replay(&self, nodes: &[Node], session: &mut Session, tail: bool) -> Result<Step, Error> {
        let changed = |guard: &Guard| match *guard { Guard::Starts(i) => session.starts_with_operator(run(nodes, i)), Guard::Ends(i) => session.ends_in_dyadic_operator(run(nodes, i)) };
        if self.guards.iter().any(changed) { return Binder::evaluate(nodes, session, tail); }
        session.execution.check(&nodes[0].span)?;
        let source = &nodes[0].span.source;
        let (mut stack, mut pending, mut cursor) = (Stack::new(), Stack::new(), nodes.len());
        for op in &self.ops {
            match op {
                Recorded::Next { leaf, shift, range } => {
                    cursor -= 1;
                    let node = &nodes[cursor];
                    let range = range.clone();
                    let (item, check) = match leaf {
                        Leaf::Value(v) => (Item { part: Part::Array(Cow::Borrowed(v)), range, shy: false }, None),
                        Leaf::Function(f) => (Item { part: Part::Function(Cow::Borrowed(f)), range, shy: false }, None),
                        Leaf::Resolve(check) => (Item::of(session.resolve(node)?.value, range, false), Some(*check)),
                        Leaf::Group(recording, check) => {
                            let (NodeKind::Group(inner) | NodeKind::Run(inner)) = &node.kind else { unreachable!() };
                            (Item::of(session.nested(inner, |session| recording.group(node, inner, session))?.value, range, false), Some(*check))
                        }
                    };
                    if check.is_some_and(|c| c != item.category()) { return resumed(nodes, session, tail, stack, pending, cursor, Some(item), false); }
                    if *shift { stack.push(item) } else { pending.push(item) }
                }
                Recorded::Unpend => { let item = pending.pop(); stack.push(item) }
                Recorded::Pend => { let item = stack.pop(); pending.push(item) }
                Recorded::Reduce(Reduction::Call { span, range, result } | Reduction::Dyadic { span, range, result }) => {
                    let (x, f, y) = if let Recorded::Reduce(Reduction::Dyadic { .. }) = op {
                        let (x, g, y) = (stack.pop(), stack.pop(), stack.pop());
                        let (x, f, _) = attached(x, g, source)?;
                        (Some(x), f, y.argument(source)?)
                    } else {
                        let (left, right) = (stack.pop(), stack.pop());
                        let (x, f) = split_call(left, source)?;
                        (x, f, right.argument(source)?)
                    };
                    if tail_call(tail, cursor, pending.len + stack.len, &f) {
                        return Ok(Step::Tail(Application { function: f.into_owned(), left: x.map(Cow::into_owned), right: y.into_owned(), span: span.clone(), unshy: false, selection: None, expression: None }));
                    }
                    let bound = f.call(x.as_deref(), &y, &mut session.at(span))?;
                    let item = Item::of(bound.value, range.clone(), bound.shy);
                    let differs = item.category() != *result;
                    stack.push(item);
                    if differs { return resumed(nodes, session, tail, stack, pending, cursor, None, true); }
                }
                Recorded::Reduce(reduction) => {
                    let (left, right) = (stack.pop(), stack.pop());
                    match reduction {
                        Reduction::Drop => stack.push(right),
                        Reduction::DropRight => stack.push(left),
                        Reduction::Strand => {
                            let (items, _, range) = strand(left, right, source)?;
                            stack.push(Item { part: Part::Strand(items), range, shy: false });
                        }
                        Reduction::Attach => {
                            let (x, f, range) = attached(left, right, source)?;
                            stack.push(Item { part: Part::Left(x, f), range, shy: false });
                        }
                        Reduction::Call { .. } | Reduction::Dyadic { .. } => unreachable!("calls match above"),
                    }
                }
            }
        }
        let item = stack.pop();
        let shy = item.shy;
        Ok(Step::Done(Bound { value: item.binding(source)?, shy, expression: None }))
    }

    /// The group or run `node`, whose members are `nodes`, evaluated by replay, as `Binder::nested` evaluates it.
    fn group(&self, node: &Node, nodes: &[Node], session: &mut Session) -> Result<Bound, Error> {
        let Step::Done(bound) = self.replay(nodes, session, false)? else { unreachable!("a group isn't in tail position") };
        Binder::group_bound(node, session, bound)
    }
}

impl Session {
    /// `nodes`, a statement or the part of one that `return_expression` evaluates, as `Binder::evaluate` gives it. Where `slot` is given, the first evaluation records the binder's steps in it, and later evaluations replay them.
    pub(super) fn statement_step(&mut self, slot: Option<&OnceLock<Option<Recording>>>, nodes: &[Node], tail: bool) -> Result<Step, Error> {
        let Some(slot) = slot else { return Binder::evaluate(nodes, self, tail) };
        match slot.get() {
            Some(Some(recording)) => recording.replay(nodes, self, tail),
            Some(None) => Binder::evaluate(nodes, self, tail),
            None => {
                let (step, recorded) = Binder::recorded(nodes, self, tail)?;
                let _ = slot.set(recorded);
                Ok(step)
            }
        }
    }
}
