//! Compiled statements, stage 3 of `meta/calls.md`. On a dfn statement's first evaluation, the binder's recorder (`Recorder`) watches each decision the binder makes: each node it resolves, each move between its stack and its pending stack, and each reduction. The recorder follows the two stacks over what each item is read from, a register or a constant, and compiles an instruction for each node that needs evaluating and each reduction (`Op`). A move between the stacks compiles to nothing, and an instruction reads a constant where the recording keeps it. Each group or run in the statement is a level (`Level`), whose instructions sit inline between an `Enter` and an `Exit`, so one loop runs the whole statement. Later evaluations run the instructions (`Recording::evaluate`), which make the same calls in the same order with no category decisions, dot-access scans or node dispatch. A recording keeps each fact about what names hold that a level's binding read (`Fact`), and runs a level's instructions only while every fact still holds. A level that dot access, a subscript, a joiner or a run that starts with an operator rewrites also keeps the nodes that the binder walked. The binder's decisions depend only on categories, so a recording checks the category of each name, group and call result against the recorded one. Where one differs, the binder of the level that holds the instruction carries on from the state that the instruction's template records (`Template`, `Binder::resumed`).
use super::*;
use super::binder::{array_of, attached, bound_right, derived, function_of, narrow, split_call, strand, tails, train, Binder, Category, Derived, Entity, Resume, Term};
use super::bindings::Address;
use super::session::Within;
use super::operators::train_call;
use std::{borrow::Cow, ops::Range, sync::OnceLock};

/// The registers that a recording holds inline while it evaluates. A recording that needs more holds them on the heap.
const INLINE: usize = 8;

/// A statement's compiled binder steps (`Op`), the constants they read, how many registers they use and where the result is, with the facts about what names hold that its binding read, the nodes that the binder walked where `Session::joined` or `Session::members` rewrote the statement, and its groups and runs (`Level`), whose steps sit inline.
#[derive(Clone, Debug)]
pub(crate) struct Recording { ops: Vec<Op>, constants: Vec<Entity>, registers: usize, result: Read, facts: Vec<Fact>, rewrite: Option<Box<[Node]>>, levels: Vec<Level> }

/// What a statement's binding read about what a name holds (`Session::path_category`): the category of the binding at the name's address (`Bindings::address`), or of the field that `keys` reach from the array it holds, as it was when the statement recorded. Dot access, joiners, and runs that start with an operator or end in a dyadic one rewrite a statement by what names hold. A recording runs a level's instructions only while each of the level's facts still holds.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Fact { pub(super) address: Address, pub(super) keys: Box<[Arc<str>]>, pub(super) category: Category }

/// Where a reading of what names hold logs what it read: a recorder's facts while a statement records, and `None` otherwise.
pub(super) type Facts<'r> = Option<&'r mut Vec<Fact>>;

impl Fact {
    /// Whether what the name holds has a different category now.
    fn changed(&self, session: &Session) -> bool {
        let bindings = &session.bindings;
        let now = if self.keys.is_empty() { Category::named(bindings.category_at(&self.address)) } else { Category::field(bindings.load(&self.address), &self.keys) };
        now != self.category
    }
}

/// Where an instruction reads an entity: a register, which the read empties, or one of the recording's constants, which it borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Read { Register(u32), Constant(u32) }

/// A group or run inside a statement, whose instructions sit between an `Enter` and an `Exit`. Levels are numbered from 1 in the order of their `Enter`s, and 0 is the statement. `node` is the group's position among the nodes of the level that holds it, `parent`, and `exit` is the position of its `Exit`. `facts` and `rewrite` are the group's own, as a statement's are. Its value goes to register `to`, with the text `range`, checked against `check`. Where its category differs, `parent` resumes from `resume`. `kind` says how the binder evaluates it.
#[derive(Clone, Debug)]
struct Level { node: u32, parent: u32, exit: u32, facts: Vec<Fact>, rewrite: Option<Box<[Node]>>, to: u32, range: Range<u32>, check: Category, resume: Box<Template>, kind: Kind }

impl Level {
    /// The nodes that the level's binder walks before any rewrite: its group's or run's, or its item's, found among `parent`, the nodes of the level that holds it.
    #[inline(always)]
    fn inner<'p>(&self, parent: &'p [Node]) -> &'p [Node] {
        match &parent[self.node as usize].kind {
            NodeKind::Group(inner) | NodeKind::Run(inner) => inner,
            NodeKind::ArrayLiteral { cells, .. } => {
                let Kind::Item { cell, start, end, .. } = self.kind else { unreachable!("a bracket literal's level is an item") };
                &cells[cell as usize][start as usize..end as usize]
            }
            _ => unreachable!("a level is a group, a run or a bracket literal's item"),
        }
    }
}

/// What a level evaluates: a group or run, which `Binder::nested` evaluates, a run that ends in a dyadic operator, which `Binder::evaluate_seeded` evaluates with its right operand, read from the `Read`, already on the binder's stack, or an item of a bracket literal, the nodes in `start..end` of the literal's cell `cell`, which `Session::item_result` evaluates and a keyed item's `key` applies to. Each counts toward the depth limit, as `Session::nested` counts what the binder evaluates.
#[derive(Clone, Copy, Debug)]
enum Kind { Group, Seeded(Read), Item { cell: u32, start: u32, end: u32, key: Option<Keyed> } }

/// The key of a keyed bracket item's level: the recording's constant that holds the key, and the position of the item's `:` in its cell.
#[derive(Clone, Copy, Debug)]
struct Keyed { constant: u32, colon: u32 }

/// The binder's state where an instruction's check fails: what its stack and its pending stack hold, bottom first, its cursor, the leaf it had just resolved, whether it had just pushed a call's result, and the `←` of an assignment it was making.
#[derive(Clone, Debug)]
struct Template { stack: Box<[Read]>, pending: Box<[Read]>, cursor: u32, next: Option<Read>, after_call: bool, assignment: Option<u32> }

/// One compiled binder step. Each writes its result to register `to`. A checked step names its level, and resumes that level's binder from `resume` where its result's category differs from the recorded `check` or `result`. `range` is the result's text.
#[derive(Clone, Debug)]
enum Op {
    /// The node at `node`, among `level`'s nodes: a name read at its `address` (`Bindings::load`), or any node evaluated by `Session::resolve`, as a name is where its address can't tell.
    Resolve { level: u32, node: u32, address: Option<Address>, to: u32, range: Range<u32>, check: Category, resume: Box<Template> },
    /// The bracket literal at `node`, among `level`'s nodes, built by `Session::bracket` from `pieces`, read in the order that `bracket` asks for its items and keys: the registers that item levels gave, and constant items and keys.
    Array { level: u32, node: u32, pieces: Box<[Read]>, to: u32, range: Range<u32>, check: Category, resume: Box<Template> },
    /// The start of a nested level. Where one of its facts no longer holds, the binder evaluates its group, and the steps after its `Exit` carry on.
    Enter { level: u32 },
    /// The end of a nested level, whose value `from` holds.
    Exit { level: u32, from: Read },
    /// The end of a bracket item's level, whose value `from` holds. The value goes into its literal whatever its category.
    ExitItem { level: u32, from: Read },
    /// A call made at the function's text `span`: `function` on `argument`, with `left` bound as its left argument where one is given. `last` marks the statement's last reduction, made with nothing left to resolve or on either stack and no assignment in progress, which returns a call of a dfn or a derived function to the caller's loop in a dfn's last statement.
    Call { level: u32, left: Option<Read>, function: Read, argument: Read, to: u32, span: Span, range: Range<u32>, result: Category, last: bool, resume: Box<Template> },
    /// A call, as `Call` makes it, of a group's train, whose register `function` holds its tines (`Recording::call_tines`). A train's call is never a tail call.
    CallTines { level: u32, left: Option<Read>, function: Read, argument: Read, to: u32, span: Span, range: Range<u32>, result: Category, resume: Box<Template> },
    /// An operator applied to its operand at `span`: a derived function, or the call that a power, `ᵀ` or `ₓ` on an array stands for.
    Derive { level: u32, operand: Read, operator: Read, to: u32, span: Span, range: Range<u32>, result: Category, resume: Box<Template> },
    /// An assignment through `Session::assign` to the target before `←`, the node at `node` among `level`'s nodes: of `value`, or with no value, `a(f)←`'s update (`Session::modify_monadic`). A target that is a plain name stores in the frame's slot for it, `local`, where it has one. `run` counts the nodes from the start of the assignment's run to `←`, and `target` those of its target. A target of more than one node must start where it started when recorded, and `level`'s binder resumes from `resume` where it doesn't. The value goes to register `to`, shy, with the text `range`, which is `←`'s.
    Assign { level: u32, value: Option<Read>, local: Option<u32>, node: u32, run: u32, target: u32, to: u32, range: Range<u32>, resume: Box<Template> },
    /// A dyadic operator bound to its right operand.
    BindRight { operator: Read, operand: Read, to: u32 },
    /// Two items side by side as a strand.
    Strand { left: Read, right: Read, to: u32 },
    /// An array bound as a function's left argument.
    Attach { left: Read, right: Read, to: u32 },
    /// Two items joined as tines of a train.
    Train { left: Read, right: Read, to: u32 },
}

impl Op {
    /// Whether the step evaluates a node or makes a call, so that its result can differ between evaluations. Facts concern names and groups, and a name, or a group that holds one, compiles to an instruction that evaluates it.
    fn computes(&self) -> bool { matches!(self, Op::Resolve { .. } | Op::Array { .. } | Op::Enter { .. } | Op::Call { .. } | Op::CallTines { .. } | Op::Derive { .. } | Op::Assign { .. }) }
}

/// A reduction that the binder reports to its recorder. A call reports once the binder has made it (`Recorder::called`).
#[derive(Clone, Debug)]
pub(super) enum Reduction {
    /// An absent `⍺` drops out.
    Drop,
    /// An absent item on the right drops out.
    DropRight,
    Strand,
    Attach,
    /// An operator applied to its operand at `span`, which gave `value`, of category `result`. `range` is its text.
    Derive { span: Span, range: Range<u32>, result: Category, value: Binding },
    /// A dyadic operator bound to its right operand, which gave the bound operator.
    BindRight(OperatorNode),
    /// Two items joined as tines of a train.
    Train,
}

/// A resolved node, until the binder shifts or holds it: a constant, a node that `Session::resolve` evaluates, a level whose `Exit` placed its value in a register, or a bracket literal, with what its items and keys are read from, in the order that `Session::bracket` asks for them.
enum Leaf { Constant(Term), Resolve(Category, Option<Address>), Register(u32), Literal(Box<[Read]>, Category) }

/// How far the recording had got: its numbers of instructions, constants and levels, and its registers in use and free. Truncating to a mark drops what came after it (`Recorder::truncate`).
#[derive(Default)]
struct Mark { ops: usize, constants: usize, levels: usize, registers: u32, free: Vec<u32> }

/// What the recorder keeps for one level as the binder walks it: the level's number, what the binder's two stacks hold, the binder's cursor, the `←` of an assignment it is making, whether a step was outside the subset that recordings cover, the facts about what names hold that its binding reads, and the nodes it walks where `Session::joined` or `Session::members` rewrote them.
#[derive(Default)]
struct Walk { level: u32, stack: Vec<Read>, pending: Vec<Read>, cursor: u32, assignment: Option<u32>, failed: bool, facts: Vec<Fact>, rewrite: Option<Box<[Node]>> }

/// A level that the recorder has entered: what it evaluates, its node's position among the nodes of the level that holds it, and how far the recording had got before it.
struct Open { kind: Kind, node: u32, start: Mark }

/// A bracket literal while the recorder records its pieces: how far the recording had got before it, what its items are read from so far, a keyed item's value whose key comes next, and whether the literal falls back to `Session::resolve`.
#[derive(Default)]
struct Literal { start: Mark, members: Vec<Member>, unkeyed: Option<Unkeyed>, failed: bool }

/// A bracket literal's item: what it's read from, or a keyed item whose value and key are both constant, whose entries the literal works out once its pieces are done (`Recorder::literal`).
enum Member { Read(Read), Keyed { cell: usize, colon: usize, key: Value, value: Value, text: Range<u32> } }

/// A keyed item's value, whose level ends once its key comes: the level's walk, how it opened and where its value is, the value where it's constant, the position of its `:` and its text.
struct Unkeyed { walk: Walk, open: Open, from: Read, constant: Option<Value>, colon: usize, text: Range<u32> }

/// What the binder's recorder keeps while the binder evaluates a statement: the instructions, constants and levels so far, the registers in use and free, the leaf just resolved until the binder shifts or holds it, the level being recorded (`Walk`), each level that holds it, with how the level it holds opened, and each bracket literal whose pieces it is recording, innermost last. Each level's instructions go straight into the statement's, between the level's `Enter` and its `Exit`, as the binder evaluates the level. `call` marks a call that `reduce` has returned and not yet made, with `derive` where a superscript on an array gave it.
#[derive(Default)]
pub(super) struct Recorder {
    ops: Vec<Op>, constants: Vec<Entity>, levels: Vec<Option<Level>>, registers: u32, free: Vec<u32>,
    /// The registers that hold a group's train as its tines (`Term::Tines`), until a register is given out again.
    trains: Vec<u32>,
    held: Option<(Leaf, Range<u32>)>, pub(super) call: bool, pub(super) derive: bool, walk: Walk, outer: Vec<(Walk, Open)>, literals: Vec<Literal>,
}

impl Recorder {
    pub(super) fn fail(&mut self) { self.walk.failed = true; }
    /// The facts about what names hold that the binding reads, which `Session::joined`, `Session::members` and the binder log.
    pub(super) fn facts(&mut self) -> &mut Vec<Fact> { &mut self.walk.facts }
    /// Notes that the binder walks `nodes`, which `Session::joined` and `Session::members` rewrote the level's nodes into.
    pub(super) fn rewritten(&mut self, nodes: Vec<Node>) { self.walk.rewrite = Some(nodes.into_boxed_slice()); }
    /// Holds the leaf for `node`, at `cursor` among the level's nodes, which resolved to `binding`.
    pub(super) fn leaf(&mut self, node: &Node, cursor: usize, binding: &Binding, session: &Session) {
        let leaf = match (&node.kind, binding) {
            (NodeKind::Literal(_) | NodeKind::Function(_) | NodeKind::Operator(_), _) => Leaf::Constant(Term::Binding(binding.clone())),
            (NodeKind::Name(name), _) => Leaf::Resolve(Category::bound(binding), Some(session.bindings.address(*name))),
            _ => Leaf::Resolve(Category::bound(binding), None),
        };
        self.walk.cursor = cursor as u32;
        self.held = Some((leaf, narrow(&node.span.range)));
    }
    /// Starts recording the group or run at `cursor` among the level's nodes, as a level of its own.
    pub(super) fn group_starts(&mut self, cursor: usize) {
        self.walk.cursor = cursor as u32;
        self.enter(Kind::Group, Vec::new());
    }
    /// Starts recording the run at `cursor` among the level's nodes, which ends in a dyadic operator, as a level whose binder starts with the run's right operand on its stack (`Binder::seeded_with`). The binder took that operand from the top of its stack, and the level reads it from where the level that holds it kept it.
    pub(super) fn seeded_starts(&mut self, cursor: usize) {
        self.walk.cursor = cursor as u32;
        // The binder's stack held the operand, so only an earlier failure leaves this one empty.
        let seed = self.walk.stack.pop();
        if seed.is_none() { self.walk.failed = true }
        self.enter(seed.map_or(Kind::Group, Kind::Seeded), seed.into_iter().collect());
    }
    /// Holds the leaf for the group or run `node`, which gave `bound`, once its level ends. A constant group or run holds the array or function it gave, and its instructions go. Any other level's `Exit` places its value in a register.
    pub(super) fn group(&mut self, node: &Node, bound: &Bound) {
        let (walk, open, from) = self.exit();
        let Some(from) = from else { self.walk.failed = true; return };
        let range = narrow(&node.span.range);
        // A run that ends in a dyadic operator also reads its right operand, which must be a constant too.
        let constant = !matches!(open.kind, Kind::Seeded(Read::Register(_))) && !self.computes(&open);
        let leaf = match &bound.value {
            value @ (Binding::Value(_) | Binding::Function(_)) if constant => { self.truncate(open.start); Leaf::Constant(Term::Binding(value.clone())) }
            value => {
                // A group whose last step builds its value as a train exits with the train's tines (`settled`). A call of its register calls them in place (`Op::CallTines`).
                let train = matches!(open.kind, Kind::Group) && matches!(self.ops.last(), Some(&Op::Train { to, .. }) if from == Read::Register(to));
                let to = self.close(walk, &open, from, range.clone(), Category::bound(value));
                if train { self.trains.push(to) }
                Leaf::Register(to)
            }
        };
        self.held = Some((leaf, range));
    }
    /// Starts recording the bracket literal at `cursor` among the level's nodes, whose pieces each record as a level (`Recorder::piece_starts`).
    pub(super) fn literal_starts(&mut self, cursor: usize) {
        self.walk.cursor = cursor as u32;
        self.literals.push(Literal { start: self.mark(), ..Literal::default() });
    }
    /// Starts recording a piece of the bracket literal being recorded, the nodes in `at` of its cell `cell`, as a level of its own.
    pub(super) fn piece_starts(&mut self, cell: usize, at: Range<usize>) {
        self.enter(Kind::Item { cell: cell as u32, start: at.start as u32, end: at.end as u32, key: None }, Vec::new());
    }
    /// Ends the level of the literal's piece, which gave `value`, with the text `text`: an item, the value of a keyed item, or the key of the value before it, as `within` says. A constant item becomes a constant, and so does a keyed item whose value and key are both constant. Any other item stays a level, which applies its key as it ends. A key must be constant, because `bracket` evaluates a key only where its value isn't absent. The literal falls back to `Session::resolve` for a key that isn't constant, for a keyed item whose value was absent, which left its key unknown, and for a piece outside the subset that recordings cover.
    pub(super) fn piece(&mut self, within: Within, value: Option<Value>, text: Range<u32>) {
        let (walk, open, from) = self.exit();
        let mut literal = self.literals.pop().expect("a piece of a bracket literal");
        let constant = !self.computes(&open);
        // Each keyed item's value has its key straight after it, unless the value was absent.
        let in_order = literal.unkeyed.is_some() == matches!(within, Within::Key);
        match from {
            Some(from) if !literal.failed && in_order => match within {
                Within::Item if constant => match value {
                    Some(v) => { self.truncate(open.start); let read = self.constant(Term::Binding(Binding::Value(v)), text); literal.members.push(Member::Read(read)) }
                    None => literal.failed = true,
                },
                Within::Item => { let to = self.close(walk, &open, from, text, Category::Value); literal.members.push(Member::Read(Read::Register(to))) }
                Within::Value { colon, named } => {
                    let unkeyed = Unkeyed { walk, open, from, constant: value.filter(|_| constant), colon, text };
                    match named { Some(key) => self.keyed(&mut literal, unkeyed, key), None => literal.unkeyed = Some(unkeyed) }
                }
                Within::Key if constant => {
                    self.truncate(open.start);
                    let unkeyed = literal.unkeyed.take().expect("the value before a key");
                    self.keyed(&mut literal, unkeyed, value.expect("a key gives an array"));
                }
                Within::Key => literal.failed = true,
            },
            _ => literal.failed = true,
        }
        self.literals.push(literal);
    }
    /// Ends `unkeyed`, a keyed item's value, now that its key `key` is known. Where the value is constant, the item becomes a constant, and its level goes. Otherwise its level keeps the key as a constant, and applies it as it ends.
    fn keyed(&mut self, literal: &mut Literal, unkeyed: Unkeyed, key: Value) {
        let Unkeyed { walk, mut open, from, constant, colon, text } = unkeyed;
        let Kind::Item { cell, key: slot, .. } = &mut open.kind else { unreachable!("a piece's level is an item") };
        let member = match constant {
            Some(value) => { let cell = *cell as usize; self.truncate(open.start); Member::Keyed { cell, colon, key, value, text } }
            None => {
                self.constants.push(Entity::new(Term::Binding(Binding::Value(key)), text.clone(), false, None));
                *slot = Some(Keyed { constant: self.constants.len() as u32 - 1, colon: colon as u32 });
                Member::Read(Read::Register(self.close(walk, &open, from, text, Category::Value)))
            }
        };
        literal.members.push(member);
    }
    /// Holds the leaf for the bracket literal `node`, which gave `value`, once its pieces are done (`Recorder::piece`). A literal whose items are all constant is a constant. A literal that falls back evaluates through `Session::resolve`, and its pieces' instructions go.
    pub(super) fn literal(&mut self, node: &Node, value: &Binding, session: &Session) -> Result<(), Error> {
        let literal = self.literals.pop().expect("a bracket literal");
        let check = Category::bound(value);
        let leaf = match value {
            _ if literal.failed || literal.unkeyed.is_some() => { self.truncate(literal.start); Leaf::Resolve(check, None) }
            Binding::Value(v) if literal.members.iter().all(|member| !matches!(member, Member::Read(Read::Register(_)))) => { self.truncate(literal.start); Leaf::Constant(Term::Binding(Binding::Value(v.clone()))) }
            _ => {
                let mut pieces = Vec::with_capacity(literal.members.len());
                for member in literal.members {
                    pieces.push(match member {
                        Member::Read(read) => read,
                        Member::Keyed { cell, colon, key, value, text } => self.constant(Term::Entries(session.keyed(node, cell, colon, &key, &value)?), text),
                    });
                }
                Leaf::Literal(pieces.into_boxed_slice(), check)
            }
        };
        self.held = Some((leaf, narrow(&node.span.range)));
        Ok(())
    }
    /// Compiles the held leaf, which the binder shifted onto its stack, or held on its pending stack when `shift` is false.
    pub(super) fn next(&mut self, shift: bool) {
        let Some((leaf, range)) = self.held.take() else { self.walk.failed = true; return };
        let read = match leaf {
            Leaf::Constant(constant) => self.constant(constant, range),
            Leaf::Resolve(check, address) => {
                let to = self.register();
                let resume = self.template(Some(Read::Register(to)));
                self.ops.push(Op::Resolve { level: self.walk.level, node: self.walk.cursor, address, to, range, check, resume });
                Read::Register(to)
            }
            Leaf::Register(to) => Read::Register(to),
            Leaf::Literal(pieces, check) => {
                for &piece in &pieces { self.release(piece) }
                let to = self.register();
                let resume = self.template(Some(Read::Register(to)));
                self.ops.push(Op::Array { level: self.walk.level, node: self.walk.cursor, pieces, to, range, check, resume });
                Read::Register(to)
            }
        };
        if shift { self.walk.stack.push(read) } else { self.walk.pending.push(read) }
    }
    /// Notes that the top of the pending stack shifts onto the stack.
    pub(super) fn unpend(&mut self) { match self.walk.pending.pop() { Some(read) => self.walk.stack.push(read), None => self.walk.failed = true } }
    /// Notes that the top of the stack goes back to the pending stack.
    pub(super) fn pend(&mut self) { match self.walk.stack.pop() { Some(read) => self.walk.pending.push(read), None => self.walk.failed = true } }
    /// Notes that the binder reads `←`, at `cursor` among the level's nodes, which starts an assignment.
    pub(super) fn assigning(&mut self, cursor: usize) { self.walk.cursor = cursor as u32; self.walk.assignment = Some(self.walk.cursor); }
    /// Compiles the assignment that the binder made: of the value on top of the stack, or with `monadic`, `a(f)←`'s update. `run` counts the nodes from the start of the assignment's run to `←`, and the binder carries on from `cursor`, where its target starts. `local` is the slot of a target that is a plain local name. The value goes to the pending stack, shy, with the text `range`.
    pub(super) fn assigned(&mut self, run: usize, cursor: usize, monadic: bool, local: Option<u32>, range: Range<u32>) {
        let resume = self.template(None);
        let Some(node) = self.walk.assignment.take() else { self.walk.failed = true; return };
        let value = if monadic { None } else {
            let Some(value) = self.walk.stack.pop() else { self.walk.failed = true; return };
            self.release(value);
            Some(value)
        };
        let to = self.register();
        let cursor = cursor as u32;
        self.ops.push(Op::Assign { level: self.walk.level, value, local, node, run: run as u32, target: node - cursor, to, range, resume });
        self.walk.pending.push(Read::Register(to));
        self.walk.cursor = cursor;
    }
    /// Compiles `reduction` of the top two items of the stack. A drop compiles to nothing, and a derive or a bind on constants to a constant.
    pub(super) fn reduce(&mut self, reduction: Reduction) {
        let (Some(left), Some(right)) = (self.walk.stack.pop(), self.walk.stack.pop()) else { self.walk.failed = true; return };
        let read = match reduction {
            Reduction::Drop => { self.release(left); right }
            Reduction::DropRight => { self.release(right); left }
            Reduction::Strand => self.op(left, right, |left, right, to| Op::Strand { left, right, to }),
            Reduction::Attach => self.op(left, right, |left, right, to| Op::Attach { left, right, to }),
            Reduction::Train => self.op(left, right, |left, right, to| Op::Train { left, right, to }),
            Reduction::BindRight(bound) => match (left, right) {
                (Read::Constant(c), Read::Constant(_)) => { let range = self.constants[c as usize].range.clone(); self.constant(Term::Binding(Binding::Operator(bound)), range) }
                _ => self.op(left, right, |operator, operand, to| Op::BindRight { operator, operand, to }),
            },
            Reduction::Derive { span, range, result, value } => match (left, right, value) {
                (Read::Constant(_), Read::Constant(_), value @ (Binding::Function(_) | Binding::Value(_))) => self.constant(Term::Binding(value), range),
                _ => self.derive(left, right, span, range, result),
            },
        };
        self.walk.stack.push(read);
    }
    /// Compiles the call that `reduce` returned, made at `span`, whose result had category `result`. `last` says whether it was the binder's last reduction, made with nothing left to resolve or on either stack and no assignment in progress, which makes it the statement's last only on the statement's own level. A call straight after the attach that gives its left argument compiles as one call with both arguments, and the call that a superscript on an array stands for compiles as its derive.
    pub(super) fn called(&mut self, span: Span, result: Category, last: bool) {
        if !std::mem::take(&mut self.call) { self.walk.failed = true; return; }
        let range = narrow(&span.range);
        let (Some(left), Some(right)) = (self.walk.stack.pop(), self.walk.stack.pop()) else { self.walk.failed = true; return };
        let read = if std::mem::take(&mut self.derive) { self.derive(left, right, span, range, result) } else {
            let (bound, function) = match self.ops.last() {
                Some(&Op::Attach { left: x, right: f, to }) if left == Read::Register(to) => { self.ops.pop(); (Some(x), f) }
                _ => (None, left),
            };
            self.release(left);
            self.release(right);
            let to = self.register();
            let resume = self.pushed(to);
            let level = self.walk.level;
            if matches!(function, Read::Register(r) if self.trains.contains(&r)) {
                self.ops.push(Op::CallTines { level, left: bound, function, argument: right, to, span, range, result, resume });
            } else {
                self.ops.push(Op::Call { level, left: bound, function, argument: right, to, span, range, result, last: last && level == 0, resume });
            }
            Read::Register(to)
        };
        self.walk.stack.push(read);
    }
    /// The recording, unless a step was outside the subset that recordings cover.
    pub(super) fn finish(self) -> Option<Recording> {
        let result = self.result(&self.walk)?;
        let levels = self.levels.into_iter().collect::<Option<Vec<_>>>()?;
        Some(Recording { ops: self.ops, constants: self.constants, registers: self.registers as usize, result, facts: self.walk.facts, rewrite: self.walk.rewrite, levels })
    }
    /// Where the level that `walk` recorded has its value, unless a step was outside the subset that recordings cover, or the binder left something unfinished.
    fn result(&self, walk: &Walk) -> Option<Read> {
        if walk.failed || self.held.is_some() || self.call || walk.assignment.is_some() || !walk.pending.is_empty() { return None }
        match walk.stack[..] { [result] => Some(result), _ => None }
    }
    /// Starts recording a level of kind `kind`, for the node at the cursor of the level being recorded, whose binder starts with `stack` on its stack. The level's instructions follow its `Enter`.
    fn enter(&mut self, kind: Kind, stack: Vec<Read>) {
        let start = self.mark();
        self.levels.push(None);
        let level = self.levels.len() as u32;
        self.ops.push(Op::Enter { level });
        let open = Open { kind, node: self.walk.cursor, start };
        let holder = std::mem::replace(&mut self.walk, Walk { level, stack, ..Walk::default() });
        self.outer.push((holder, open));
    }
    /// Ends the level being recorded and goes back to the level that holds it, giving the ended level's walk, how it opened, and where its value is (`Recorder::result`).
    fn exit(&mut self) -> (Walk, Open, Option<Read>) {
        let (holder, open) = self.outer.pop().expect("a level that the recorder entered");
        let walk = std::mem::replace(&mut self.walk, holder);
        let from = self.result(&walk);
        (walk, open, from)
    }
    /// Places the value of the level that `walk` recorded, which `from` holds, in a new register at the level's `Exit`, and completes the level that `open` opened, with the text `range` and the category `check`. Where its value's category differs, the level that holds it resumes from its state now.
    fn close(&mut self, walk: Walk, open: &Open, from: Read, range: Range<u32>, check: Category) -> u32 {
        let to = self.register();
        let level = walk.level;
        let item = matches!(open.kind, Kind::Item { .. });
        self.ops.push(if item { Op::ExitItem { level, from } } else { Op::Exit { level, from } });
        self.release(from);
        // An item's value goes into its literal whatever its category, so an item's level resumes nothing around it.
        let resume = self.template((!item).then_some(Read::Register(to)));
        self.levels[level as usize - 1] = Some(Level { node: open.node, parent: self.walk.level, exit: self.ops.len() as u32 - 1, facts: walk.facts, rewrite: walk.rewrite, to, range, check, resume, kind: open.kind });
        to
    }
    fn mark(&self) -> Mark { Mark { ops: self.ops.len(), constants: self.constants.len(), levels: self.levels.len(), registers: self.registers, free: self.free.clone() } }
    /// Drops the instructions, constants, levels and registers that the recording gained after `mark`: a constant group's, or those of a literal that falls back.
    fn truncate(&mut self, mark: Mark) {
        self.ops.truncate(mark.ops);
        self.constants.truncate(mark.constants);
        self.levels.truncate(mark.levels);
        (self.registers, self.free) = (mark.registers, mark.free);
    }
    /// Whether an instruction of the level that `open` opened, after its `Enter`, evaluates a node or makes a call. A level where none does gives the same binding every time.
    fn computes(&self, open: &Open) -> bool { self.ops[open.start.ops + 1..].iter().any(Op::computes) }
    /// Reads `term`, kept as a constant with its text `range`.
    fn constant(&mut self, term: Term, range: Range<u32>) -> Read {
        self.constants.push(Entity::new(term, range, false, None));
        Read::Constant(self.constants.len() as u32 - 1)
    }
    /// A register that holds nothing the binder still needs.
    fn register(&mut self) -> u32 {
        let r = self.free.pop().unwrap_or_else(|| { self.registers += 1; self.registers - 1 });
        self.trains.retain(|&t| t != r);
        r
    }
    /// Frees `read`'s register, whose item a reduction used up or dropped.
    fn release(&mut self, read: Read) { if let Read::Register(r) = read { self.free.push(r) } }
    /// `left` and `right` reduced by the instruction that `op` makes, into a register.
    fn op(&mut self, left: Read, right: Read, op: impl FnOnce(Read, Read, u32) -> Op) -> Read {
        self.release(left);
        self.release(right);
        let to = self.register();
        self.ops.push(op(left, right, to));
        Read::Register(to)
    }
    /// A derive at run time, checked against `result`.
    fn derive(&mut self, operand: Read, operator: Read, span: Span, range: Range<u32>, result: Category) -> Read {
        self.release(operand);
        self.release(operator);
        let to = self.register();
        let resume = self.pushed(to);
        self.ops.push(Op::Derive { level: self.walk.level, operand, operator, to, span, range, result, resume });
        Read::Register(to)
    }
    /// The binder's state now, with `next` just resolved, or with no new leaf, as where it makes an assignment.
    fn template(&self, next: Option<Read>) -> Box<Template> {
        let walk = &self.walk;
        Box::new(Template { stack: walk.stack.as_slice().into(), pending: walk.pending.as_slice().into(), cursor: walk.cursor, next, after_call: false, assignment: walk.assignment })
    }
    /// The binder's state once it has pushed the result in register `to`.
    fn pushed(&self, to: u32) -> Box<Template> {
        let walk = &self.walk;
        let stack = walk.stack.iter().copied().chain([Read::Register(to)]).collect();
        Box::new(Template { stack, pending: walk.pending.as_slice().into(), cursor: walk.cursor, next: None, after_call: true, assignment: walk.assignment })
    }
}

/// `entity` as a group's value: a strand, or a left argument with its function, becomes the array or function it stands for, as `Entity::value` makes it. A train stays as its tines (`Term::Tines`) until something needs its function.
fn settled(entity: Entity, source: &Arc<Source>) -> Result<Entity, Error> {
    let (range, shy) = (entity.range.clone(), entity.shy);
    Ok(match entity.term {
        Term::Train(tines) => Entity::new(Term::Tines(tines), range, shy, None),
        Term::Strand(_) | Term::Left(..) => Entity::of(entity.value(source)?, range, shy),
        _ => entity,
    })
}

/// The entity in register `r`, which the read empties.
#[inline(always)]
fn taken(registers: &mut [Option<Entity>], r: u32) -> Entity { registers[r as usize].take().expect("an instruction reads a register that an earlier one wrote") }

/// Writes `entity` to register `to`, and gives whether its category differs from the recorded `check`.
#[inline(always)]
fn put(registers: &mut [Option<Entity>], to: u32, entity: Entity, check: Category) -> bool {
    let differs = entity.category() != check;
    registers[to as usize] = Some(entity);
    differs
}

/// What the binder gave once it took over from a recording: the position of the step to carry on from, or the statement's step.
enum Resumed { Continue(usize), Done(Step) }

/// Why the binder takes a level over: a step's check failed, with the step's template, one of the level's facts no longer holds, or its group's value has another category than the recorded one.
enum Over<'t> { Failed(&'t Template), Changed, Exited(Entity) }

impl Recording {
    /// `nodes` evaluated by the compiled steps, which gives what `Binder::evaluate` gives. It checks for an interrupt once, when it starts, and calls check for themselves. One loop runs every step, so a level adds no interrupt check or stack growth. Each level still counts toward the depth limit while it runs, as `Session::nested` counts what the binder evaluates.
    pub(super) fn evaluate(&self, nodes: &[Node], session: &Session, tail: bool) -> Result<Step, Error> {
        if self.facts.iter().any(|fact| fact.changed(session)) { return Binder::evaluate(nodes, session, tail); }
        let nodes = self.rewrite.as_deref().unwrap_or(nodes);
        crate::execution::check().at(&nodes[0].span)?;
        let mut inline: [Option<Entity>; INLINE];
        let mut heap: Vec<Option<Entity>>;
        let registers: &mut [Option<Entity>] = if self.registers <= INLINE { inline = Default::default(); &mut inline } else {
            heap = std::iter::repeat_with(|| None).take(self.registers).collect();
            &mut heap
        };
        if self.levels.is_empty() { return self.execute(session, tail, registers, nodes) }
        // An error inside a level leaves the counts of its open levels behind, so the depth goes back to what it was.
        let depth = session.depth.get();
        let result = self.execute(session, tail, registers, nodes);
        session.depth.set(depth);
        result
    }

    /// The nodes of level `level`: the statement's, or a nested level's, found from the statement through the levels that hold it. A level that a rewrite changed has the rewrite's nodes.
    fn nodes<'p>(&'p self, level: u32, statement: &'p [Node]) -> &'p [Node] {
        if level == 0 { return statement }
        let held = &self.levels[level as usize - 1];
        held.rewrite.as_deref().unwrap_or_else(|| held.inner(self.nodes(held.parent, statement)))
    }

    /// Runs the steps over the statement's nodes, `statement`.
    fn execute<'p>(&'p self, session: &Session, tail: bool, registers: &mut [Option<Entity>], statement: &'p [Node]) -> Result<Step, Error> {
        let source = &statement[0].span.source;
        let mut ops = self.ops.iter();
        while let Some(op) = ops.next() {
            // A step that passes its check carries on with the next. Otherwise the binder takes its level over.
            let (level, over) = match op {
                Op::Resolve { level, node, address, to, range, check, resume } => {
                    let value = match address.as_ref().and_then(|address| session.bindings.load(address)) { Some(value) => session.read_binding(value)?, None => session.resolve(&self.nodes(*level, statement)[*node as usize])?.value };
                    if !put(registers, *to, Entity::of(value, range.clone(), false), *check) { continue }
                    (*level, Over::Failed(resume))
                }
                Op::Array { level, node, pieces, to, range, check, resume } => {
                    if !self.array(registers, &self.nodes(*level, statement)[*node as usize], pieces, *to, range, *check)? { continue }
                    (*level, Over::Failed(resume))
                }
                Op::Enter { level } => {
                    let held = &self.levels[*level as usize - 1];
                    // A level counts toward the depth limit until it exits, as `Session::nested` counts what the binder evaluates.
                    if !session.deeper() { return Err(depth_error(&held.inner(self.nodes(held.parent, statement))[0].span)) }
                    if !held.facts.iter().any(|fact| fact.changed(session)) { continue }
                    (*level, Over::Changed)
                }
                Op::Exit { level, from } => {
                    let held = &self.levels[*level as usize - 1];
                    let mut entity = settled(self.owned(registers, *from), source)?;
                    (entity.range, entity.shy) = (held.range.clone(), false);
                    // A value of the recorded category passes `Binder::group_bound`'s check, as it did when recorded: the level's facts hold, so its members are the same.
                    if entity.category() == held.check { registers[held.to as usize] = Some(entity); session.ascend(); continue }
                    (*level, Over::Exited(entity))
                }
                Op::ExitItem { level, from } => {
                    let held = &self.levels[*level as usize - 1];
                    self.item_exit(session, registers, held, *from, self.nodes(held.parent, statement), source)?;
                    continue
                }
                Op::Call { level, left, function, argument, to, span, range, result, last, resume } => {
                    let (x, f) = match left {
                        Some(x) => (Some(self.argument(registers, *x, source)?), self.called(registers, *function, source)?),
                        None => split_call(self.read(registers, *function), source)?,
                    };
                    let y = self.argument(registers, *argument, source)?;
                    if tail && *last && tails(&f) {
                        return Ok(Step::Tail(Application { function: f.into_owned(), left: x.map(Cow::into_owned), right: y.into_owned(), span: span.clone(), unshy: false, expression: None }));
                    }
                    let bound = f.call(x.as_deref(), &y, span)?;
                    if !put(registers, *to, Entity::of(bound.value, range.clone(), bound.shy), *result) { continue }
                    (*level, Over::Failed(resume))
                }
                Op::CallTines { level, left, function, argument, to, span, range, result, resume } => {
                    let bound = self.call_tines(registers, *left, *function, *argument, span, source)?;
                    if !put(registers, *to, Entity::of(bound.value, range.clone(), bound.shy), *result) { continue }
                    (*level, Over::Failed(resume))
                }
                Op::Derive { level, operand, operator, to, span, range, result, resume } => {
                    let (operand, operator) = (self.value(registers, *operand, source)?, self.value(registers, *operator, source)?);
                    let entity = match derived(operand, operator, span)? {
                        Derived::Term(binding) => Entity::of(binding, range.clone(), false),
                        Derived::Call(f, x, y) => { let bound = f.call(x.as_ref(), &y, span)?; Entity::of(bound.value, range.clone(), bound.shy) }
                    };
                    if !put(registers, *to, entity, *result) { continue }
                    (*level, Over::Failed(resume))
                }
                Op::Assign { level, value, local, node, run, target, to, range, resume } => {
                    let nodes = self.nodes(*level, statement);
                    let i = *node as usize;
                    let (begin, cursor) = (i - *run as usize, i - *target as usize);
                    // A target of more than one node starts where what its names hold puts it (`Session::assignment_start`).
                    if *run > 1 && begin + session.assignment_start(&nodes[begin..i])? != cursor { (*level, Over::Failed(resume)) } else {
                        // The value's category was checked where it was made, so it's neither absent nor no result.
                        let value = value.map(|value| self.value(registers, value, source)).transpose()?;
                        let value = session.assigned(&nodes[cursor..i], value, *local, &nodes[i].span)?;
                        registers[*to as usize] = Some(Entity::of(value, range.clone(), true));
                        continue
                    }
                }
                Op::BindRight { operator, operand, to } => {
                    let operator = self.owned(registers, *operator);
                    let range = operator.range.clone();
                    registers[*to as usize] = Some(Entity::of(Binding::Operator(bound_right(operator.value(source)?, self.value(registers, *operand, source)?)), range, false));
                    continue
                }
                Op::Strand { left, right, to } => {
                    let (items, _, range) = strand(self.owned(registers, *left), self.owned(registers, *right), source)?;
                    registers[*to as usize] = Some(Entity::new(Term::Strand(items), range, false, None));
                    continue
                }
                Op::Attach { left, right, to } => {
                    let (x, f, range) = attached(self.read(registers, *left), self.read(registers, *right), source)?;
                    registers[*to as usize] = Some(Entity::new(Term::Left(x.into_owned(), f.into_owned()), range, false, None));
                    continue
                }
                Op::Train { left, right, to } => {
                    let (left, right) = (self.read(registers, *left), self.read(registers, *right));
                    let range = left.range.clone();
                    registers[*to as usize] = Some(Entity::new(Term::Train(train(left, right, source)?), range, false, None));
                    continue
                }
            };
            match self.taken_over(level, over, session, tail, registers, statement)? { Resumed::Continue(next) => ops = self.ops[next..].iter(), Resumed::Done(step) => return Ok(step) }
        }
        let shy = match self.result { Read::Register(r) => registers[r as usize].as_ref().is_some_and(|entity| entity.shy), Read::Constant(_) => false };
        Ok(Step::Done(Bound { value: self.value(registers, self.result, source)?, shy, expression: None }))
    }

    /// Builds the bracket literal `node` from `pieces`, its items as their levels prepared them (`Recording::prepared`) and its constant items, writes it to register `to` with the text `range`, and gives whether its category differs from `check`. Only the literal's assembly is left to do (`Session::record_literal`, `Session::list_literal`).
    #[inline(never)]
    fn array(&self, registers: &mut [Option<Entity>], node: &Node, pieces: &[Read], to: u32, range: &Range<u32>, check: Category) -> Result<bool, Error> {
        let NodeKind::ArrayLiteral { record, .. } = &node.kind else { unreachable!("a bracket literal") };
        let value = if *record {
            let mut entries = Vec::with_capacity(pieces.len());
            for &piece in pieces {
                match self.read(registers, piece) {
                    Cow::Owned(Entity { term: Term::Entries(more), .. }) => entries.extend(more),
                    Cow::Borrowed(Entity { term: Term::Entries(more), .. }) => entries.extend_from_slice(more),
                    Cow::Owned(Entity { term: Term::Binding(Binding::Value(value)), .. }) => entries.push((None, value)),
                    Cow::Borrowed(Entity { term: Term::Binding(Binding::Value(value)), .. }) => entries.push((None, value.clone())),
                    _ => {}
                }
            }
            Session::record_literal(node, entries)?
        } else {
            let mut arrays = Vec::with_capacity(pieces.len());
            for &piece in pieces { if let Term::Binding(Binding::Value(value)) = self.owned(registers, piece).term { arrays.push(value) } }
            Session::list_literal(node, arrays)?
        };
        Ok(put(registers, to, Entity::of(value, range.clone(), false), check))
    }

    /// Ends the level of a bracket item, `held`, whose value `from` holds: the value goes into register `held.to` at once, prepared for its literal (`Recording::prepared`), so the items act and fail in order. `parent` holds the nodes of the level that holds the literal.
    #[inline(never)]
    fn item_exit(&self, session: &Session, registers: &mut [Option<Entity>], held: &Level, from: Read, parent: &[Node], source: &Arc<Source>) -> Result<(), Error> {
        let value = self.value(registers, from, source)?;
        registers[held.to as usize] = Some(self.prepared(session, held, parent, value)?);
        session.ascend();
        Ok(())
    }

    /// A bracket item's value as its literal takes it, as `Session::bracket` prepares it: nothing for an absent `⍺`, the entries of a keyed item (`Session::keyed`), or an array (`session::listed`). `held` is the item's level, and `parent` holds the nodes of the level that holds the literal.
    fn prepared(&self, session: &Session, held: &Level, parent: &[Node], value: Binding) -> Result<Entity, Error> {
        let Kind::Item { cell, key, .. } = held.kind else { unreachable!("a bracket item's level") };
        let term = match (super::session::listed(value, &held.inner(parent)[0].span)?, key) {
            (None, _) => Term::Binding(Binding::Absent),
            (Some(value), None) => Term::Binding(Binding::Value(value)),
            (Some(value), Some(Keyed { constant, colon })) => {
                let Term::Binding(Binding::Value(key)) = &self.constants[constant as usize].term else { unreachable!("a key is an array") };
                Term::Entries(session.keyed(&parent[held.node as usize], cell as usize, colon as usize, key, &value)?)
            }
        };
        Ok(Entity::new(term, held.range.clone(), false, None))
    }

    /// The call of a group's train, which register `function` holds as its tines: by its tines where `train_call` can call them in place, and otherwise as its function. A level that the binder took over holds its train as a function, which calls as any function does. A train's call is never a tail call.
    #[inline(never)]
    fn call_tines(&self, registers: &mut [Option<Entity>], left: Option<Read>, function: Read, argument: Read, span: &Span, source: &Arc<Source>) -> Result<Bound, Error> {
        let x = match left { Some(x) => Some(self.argument(registers, x, source)?), None => None };
        let f = self.read(registers, function);
        let y = self.argument(registers, argument, source)?;
        if let Term::Tines(tines) = &f.term && let Some(bound) = train_call(tines, x.as_deref(), &y, span) { return bound; }
        let (x, f) = match x { Some(x) => (Some(x), function_of(f, source)?), None => split_call(f, source)? };
        f.call(x.as_deref(), &y, span)
    }

    /// The entity that `read` gives: a register's, which it takes, or a constant, which it borrows.
    #[inline(always)]
    fn read<'p>(&'p self, registers: &mut [Option<Entity>], read: Read) -> Cow<'p, Entity> {
        match read {
            Read::Register(r) => Cow::Owned(taken(registers, r)),
            Read::Constant(c) => Cow::Borrowed(&self.constants[c as usize]),
        }
    }

    /// `read`'s entity as an argument, as `array_of` gives it, with a register's entity taken without a `Cow` around it.
    #[inline(always)]
    fn argument<'p>(&'p self, registers: &mut [Option<Entity>], read: Read, source: &Arc<Source>) -> Result<Cow<'p, Value>, Error> {
        match read { Read::Register(r) => taken(registers, r).array(source).map(Cow::Owned), Read::Constant(c) => array_of(Cow::Borrowed(&self.constants[c as usize]), source) }
    }

    /// `read`'s entity as a called function, as `function_of` gives it, with a register's entity taken without a `Cow` around it.
    #[inline(always)]
    fn called<'p>(&'p self, registers: &mut [Option<Entity>], read: Read, source: &Arc<Source>) -> Result<Cow<'p, Function>, Error> {
        match read { Read::Register(r) => taken(registers, r).function(source).map(Cow::Owned), Read::Constant(c) => function_of(Cow::Borrowed(&self.constants[c as usize]), source) }
    }

    /// The entity that `read` gives, owned: a register's, which the read empties, or a copy of a constant.
    #[inline(always)]
    fn owned(&self, registers: &mut [Option<Entity>], read: Read) -> Entity {
        match read { Read::Register(r) => taken(registers, r), Read::Constant(c) => self.constants[c as usize].clone() }
    }

    /// `read`'s entity's value (`Entity::value`), with only a constant's binding copied.
    #[inline(always)]
    fn value(&self, registers: &mut [Option<Entity>], read: Read, source: &Arc<Source>) -> Result<Binding, Error> {
        match read {
            Read::Register(r) => taken(registers, r).value(source),
            Read::Constant(c) => match &self.constants[c as usize] { Entity { term: Term::Binding(value), .. } => Ok(value.clone()), constant => constant.clone().value(source) },
        }
    }

    /// The binder of the level whose nodes are `nodes`, carrying on from the state that `template` records.
    #[cold]
    fn resumed(&self, template: &Template, nodes: &[Node], session: &Session, tail: bool, registers: &mut [Option<Entity>]) -> Result<Step, Error> {
        let mut entity = |read: &Read| self.owned(registers, *read);
        let stack = template.stack.iter().map(&mut entity).collect();
        let pending = template.pending.iter().map(&mut entity).collect();
        let next = template.next.as_ref().map(&mut entity);
        Binder::resumed(nodes, session, tail, Resume { stack, pending, cursor: template.cursor as usize, next, after_call: template.after_call, assignment: template.assignment.map(|at| at as usize) })
    }

    /// Carries on once the binder takes level `level` over, as `over` says why: from a failed step's template, from the group's start where a fact no longer holds, or with the group's value where its category differs. Where the group's value has the recorded category, it goes to the level's register, and the steps after its `Exit` carry on. Otherwise the level that holds it resumes from its template, and so on out to the statement, whose step ends the evaluation.
    #[cold]
    #[inline(never)]
    fn taken_over<'p>(&'p self, mut level: u32, over: Over<'_>, session: &Session, tail: bool, registers: &mut [Option<Entity>], statement: &'p [Node]) -> Result<Resumed, Error> {
        let mut step = match over {
            Over::Failed(template) => self.resumed(template, self.nodes(level, statement), session, tail && level == 0, registers)?,
            Over::Changed => {
                let held = &self.levels[level as usize - 1];
                let inner = held.inner(self.nodes(held.parent, statement));
                match held.kind {
                    Kind::Group | Kind::Item { .. } => Binder::evaluate(inner, session, false)?,
                    Kind::Seeded(seed) => Step::Done(Binder::seeded_with(inner, session, self.owned(registers, seed), None)?),
                }
            }
            Over::Exited(entity) => Step::Done(Bound { value: entity.value(&statement[0].span.source)?, shy: false, expression: None }),
        };
        while level != 0 {
            let Step::Done(bound) = step else { unreachable!("a group isn't in tail position") };
            let held = &self.levels[level as usize - 1];
            let parent = self.nodes(held.parent, statement);
            // The binder has given the level's value, so evaluation is back in the level that holds it.
            session.ascend();
            let bound = match held.kind {
                Kind::Group => Binder::group_bound(&parent[held.node as usize], session, bound)?,
                Kind::Seeded(_) => bound,
                // An item's value goes into its literal whatever its category, so the steps after the item's `ExitItem` carry on.
                Kind::Item { .. } => {
                    registers[held.to as usize] = Some(self.prepared(session, held, parent, bound.value)?);
                    return Ok(Resumed::Continue(held.exit as usize + 1));
                }
            };
            let entity = Entity::of(bound.value, held.range.clone(), false);
            if !put(registers, held.to, entity, held.check) { return Ok(Resumed::Continue(held.exit as usize + 1)); }
            step = self.resumed(&held.resume, parent, session, tail && held.parent == 0, registers)?;
            level = held.parent;
        }
        Ok(Resumed::Done(step))
    }
}

/// The recording in a statement's `slot` once the statement has compiled, or `None` before its first evaluation and for a statement that doesn't compile.
pub(super) fn compiled(slot: Option<&OnceLock<Option<Recording>>>) -> Option<&Recording> { slot?.get()?.as_ref() }

impl Session {
    /// `nodes`, a statement or the part of one that `return_expression` evaluates, as `Binder::evaluate` gives it. Where `slot` is given, the first evaluation compiles the binder's steps into it, and later evaluations run them.
    pub(super) fn statement_step(&self, slot: Option<&OnceLock<Option<Recording>>>, nodes: &[Node], tail: bool) -> Result<Step, Error> {
        if let Some(recording) = compiled(slot) { return recording.evaluate(nodes, self, tail); }
        match slot {
            Some(slot) if slot.get().is_none() => { let (step, recorded) = Binder::recorded(nodes, self, tail)?; let _ = slot.set(recorded); Ok(step) }
            _ => Binder::evaluate(nodes, self, tail),
        }
    }
}
