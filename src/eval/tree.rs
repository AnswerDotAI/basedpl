//! The binder's uncached register instructions and value constructors. Each statement compiles against its current categories, then executes once. Graph replay bypasses this executor.
use super::binder::Category;
use super::bindings::{Address, Callable};
use super::*;
use crate::{array::Widening, primitive::Notation, syntax::{Definition, DefinitionKind, Letter}};
use std::{borrow::Cow, ops::Range};

/// The statement executor's inline registers; larger statements use heap storage.
const INLINE: usize = 8;

/// Uncached instructions, constants, registers and role facts for one statement, including its nested runs and groups.
#[derive(Clone, Debug)]
pub(crate) struct StatementCode {
    pub(super) ops: Vec<Op>, pub(super) constants: Vec<Entity>, pub(super) registers: usize, pub(super) result: Read, pub(super) call: bool,
    pub(super) facts: Vec<Fact>, pub(super) span: Span, pub(super) levels: Vec<Level>
}

/// An assignment's target, as its statement compiled it (`Compiler::assigned_target`): what it assigns to, how many index values a run of it keeps (`Split::slot`), the text of its nodes, `span`, and of its first node, `at`. Running the assignment compiles nothing and reads no name's role.
#[derive(Clone, Debug)]
pub(super) struct Target { pub(super) to: To, pub(super) slots: usize, pub(super) span: Span, pub(super) at: Span }

/// What an assignment's target assigns to, with the text of what it names.
#[derive(Clone, Debug)]
pub(super) enum To {
    /// A name: it takes the value, or for a modified assignment, the array that it holds is updated.
    Name(Name, Span),
    /// `⎕`, which shows the value.
    Output(Span),
    /// Brackets of targets, each with the key that a pattern reads and the text of its first node.
    Names(Vec<(Option<Arc<str>>, To, Span)>, Span),
    /// `f∂←` where the flag holds, otherwise `f⁻¹←`: a rule for the function that the name holds.
    Rule(Name, bool, Span),
    /// A selection from an array that a name holds.
    Selected(Layer),
    /// A target whose error the assignment raises when it runs.
    Invalid(Error),
}

/// A target expression compiled around its root name. The resulting graph goes through the shared selection walk.
#[derive(Clone, Debug)]
pub(super) struct Layer {
    pub(super) split: Option<Split>, pub(super) prefix: Option<Result<StatementCode, Error>>, pub(super) last: Last, pub(super) last_at: Span,
    pub(super) span: Span, pub(super) at: Span
}

/// A selecting primitive and its index expression, evaluated once per assignment. `simple` marks a single array operand; `keyed` allows creation of missing keyed items.
#[derive(Clone, Debug)]
pub(super) struct Split {
    pub(super) p: Primitive, pub(super) index: Result<StatementCode, Error>, pub(super) slot: usize, pub(super) keyed: bool, pub(super) simple: bool,
    pub(super) unit: bool, pub(super) container: Box<Layer>, pub(super) at: Span, pub(super) p_at: Span
}

impl Split {
    /// Whether a run takes this split: one array stands before `p`, or a plain assignment evaluated the keys before it (`Session::extend`).
    pub(super) fn taken(&self, run: &[Option<Value>]) -> bool { self.simple || self.keyed && run[self.slot].is_some() }
}

/// A layer's last node: the root array's name, the split's container where the container is that node alone, a layer of its own, or a node that can't end a selection, whose error the assignment raises.
#[derive(Clone, Debug)]
pub(super) enum Last { Name(Name, Span), Container, Layer(Box<Layer>), Invalid(Error) }

/// A category or postfix role that binding read before this statement ran.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Fact { pub(super) address: Address, pub(super) keys: Box<[Arc<str>]>, pub(super) role: (Category, bool) }

pub(super) type Facts<'r> = Option<&'r mut Vec<Fact>>;

impl Fact {
    pub(super) fn new(address: Address, keys: Box<[Arc<str>]>, role: (Category, bool)) -> Self { Self { address, keys, role } }
    fn changed(&self, session: &Session) -> Result<bool, Error> {
        let bindings = &session.bindings;
        let current = if self.keys.is_empty() { bindings.role_at(&self.address)?.unwrap_or(super::binder::UNBOUND) }
        else { super::binder::field_role(bindings.load(&self.address)?, &self.keys) };
        Ok(current != self.role)
    }
}

/// Where an instruction reads an entity: a register, which the read empties, or one of the statement's constants, which it borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Read { Register(u32), Constant(u32) }

/// A group or run inside a statement, whose instructions sit between an `Enter` and an `Exit`. Levels are numbered from 1 in the order of their `Enter`s, and 0 is the statement. Its value goes to register `to`, with the text `range` and the category `check` that the compiler fixed. `span` is its first node's text, where errors in it point. `kind` says what it evaluates.
#[derive(Clone, Debug)]
pub(super) struct Level { pub(super) to: u32, pub(super) range: Range<u32>, pub(super) span: Span, pub(super) check: Category, pub(super) kind: Kind }

/// What a level evaluates: a group or run, or an item, in cell `cell`, of the bracket literal `literal`, with how a keyed item's level makes its entries (`Keyed`). Each counts toward the depth limit while it runs, as `Session::nested` counts what it evaluates.
#[derive(Clone, Debug)]
pub(super) enum Kind { Group, Item { literal: Arc<Node>, cell: u32, key: Option<Keyed> } }

/// How the level that finishes a keyed bracket item makes the item's entries (`Session::keyed`), as soon as its value and key are both ready. The value evaluates first. `Key`: the level computes the value, and the recording's constant `constant` holds the key. `Value`: the level computes the key, and `value` reads the value. `colon` is the position of the item's `:` in its cell.
#[derive(Clone, Copy, Debug)]
pub(super) enum Keyed { Key { constant: u32, colon: u32 }, Value { value: Read, colon: u32 } }

/// What a name's read does where what the name holds has another category than the compiler fixed: takes the fixed category, where an assignment earlier in the statement bound the name (`Fit`), or fails, because the role changed while the statement ran (`Fail`, `role_changed`). A role that changed before the statement started fails the statement's check of its facts as it starts (`StatementCode::holds`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mismatch { Fit, Fail }

/// One compiled binder step. Each writes its result to register `to`. A step that checks its value's category against the compiled `check` or `result` fits a value of another category to it (`StatementCode::execute`). `range` is the result's text.
#[derive(Clone, Debug)]
pub(super) enum Op {
    /// A read of `name` at its `address` (`Bindings::load`). With `take`, the read is the dfn's last read of a local, which takes the binding out of the frame (`Bindings::take_at`). A name that nothing binds fails (`unbound`). A value whose category isn't `check` does what `on` says.
    Load { name: Name, address: Address, take: bool, to: u32, range: Range<u32>, check: Category, on: Mismatch },
    /// A number literal, parsed under the settings in force: a raw value, or at an operand position its `constant`.
    Number { text: Option<&'static str>, mark: Option<crate::number::Mark>, empty: bool, to: u32, range: Range<u32>, constant: Option<Constant> },
    /// The bracket literal `literal`, assembled from `pieces` (`assembled`): the registers that its items' levels gave, and its constant items and entries, in the literal's order.
    Array { literal: Arc<Node>, pieces: Box<[Read]>, to: u32, range: Range<u32>, check: Category },
    /// The start of a nested level, which counts toward the depth limit until its `Exit`.
    Enter { level: u32 },
    /// The end of a nested level, whose value `from` holds.
    Exit { level: u32, from: Read },
    /// The end of a bracket item's level, whose value `from` holds. The value goes into its literal whatever its category.
    ExitItem { level: u32, from: Read },
    /// A call made at the function's text `span`: `function` on `argument`, with `left` bound as its left argument where one is given. `last` marks the statement's last reduction, made with nothing left to compile or on either stack and no assignment in progress. It returns a call of a dfn or a derived function to the caller's loop in a dfn's last statement, and it alone can give no result.
    Call { left: Option<Read>, function: Read, argument: Read, to: u32, span: Span, range: Range<u32>, result: Category, last: bool },
    /// An operator applied to its operand at `span`: a derived function (`derived`), or, for a name that holds a postfix operator, the call that it stands for on an array (`Postfix::of`). With `call`, a function or operator atom that a call gave as the operand is that function or operator (`called_as`).
    Derive { operand: Read, operator: Read, to: u32, span: Span, range: Range<u32>, result: Category, call: bool },
    /// An assignment through `Session::assigned` to `target`, as the statement compiled it (`Target`), whose `←` is at `span`: of `value`, or with no value, `a(f)←`'s update. Register `modifier` holds a modified assignment's function, which compiles before this as any function does. A target that is a plain name stores in the frame's slot for it, `local`, where it has one. The value goes to register `to`, shy, with the text `range`, which is `←`'s. With `call`, a function or operator atom that a call gave is stored as that function or operator, and the register keeps it as a value.
    Assign { value: Option<Read>, local: Option<u32>, target: Target, span: Span, modifier: Option<Read>, to: u32, range: Range<u32>, call: bool },
    /// A dyadic operator bound to its right operand. With `call`, a function or operator atom that a call gave as the operand is that function or operator.
    BindRight { operator: Read, operand: Read, to: u32, call: bool },
    /// Two items side by side as a strand.
    Strand { left: Read, right: Read, to: u32 },
    /// An array bound as a function's left argument.
    Attach { left: Read, right: Read, to: u32 },
    /// Two items joined as tines of a train.
    Train { left: Read, right: Read, to: u32 },
    /// A dfn, read where it's written: a function or an operator that closes over the running frame (`closure`).
    Closure { definition: Arc<Definition>, to: u32, range: Range<u32>, check: Category },
    /// A record scope compiled when entered, using its current field categories.
    Scope { root: Read, body: Arc<[Node]>, to: u32, range: Range<u32> },
    /// A read of `⎕`: a line of input, an effect through dispatch (`Act::Input`).
    Input { to: u32, range: Range<u32> },
    /// A postfix that the compiler compiled for its operand's role (`Postfix`): the call that it stands for on an array, or `ᵘ`'s scalar holding a function (`postfixed`), at `span`.
    Postfix { postfix: Postfix, operand: Read, to: u32, span: Span, range: Range<u32>, result: Category },
    /// An error found as the statement compiled, raised where the statement reaches it, so the effects to its right run first.
    Fail { error: Error },
}


/// A literal number's constant at an operand position (target part 4): its literal node, made under the numeric settings it holds and made again when they change.
#[derive(Debug, Default)]
pub(super) struct Constant(std::sync::Mutex<Option<(Numeric, Value)>>);
impl Clone for Constant {
    fn clone(&self) -> Self { Self(std::sync::Mutex::new(self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone())) }
}
impl Constant {
    /// The literal node for `numeric`, made from `make`'s value where the cached node has other settings.
    fn get(&self, numeric: Numeric, make: impl FnOnce() -> Result<Value, Error>) -> Result<Value, Error> {
        let mut cached = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((made, value)) = &*cached && *made == numeric { return Ok(value.clone()); }
        let value = Value::constant(make()?);
        *cached = Some((numeric, value.clone()));
        Ok(value)
    }
}
/// How far the compilation had got: its numbers of instructions, constants and levels, and its registers in use and free. Truncating to a mark drops what came after it (`Compiler::truncate`).
#[derive(Default)]
pub(super) struct Mark { pub(super) ops: usize, pub(super) constants: usize, pub(super) levels: usize, pub(super) registers: u32, pub(super) free: Vec<u32> }

/// `entity` as a group's value: an unfinished term (a strand, a bound left argument, a train, a postfix) becomes the array, function or operator that it stands for (`Entity::value`).
fn settled(entity: Entity, source: &Arc<Source>) -> Result<Entity, Error> {
    let (range, shy) = (entity.range.clone(), entity.shy);
    Ok(Entity::of(entity.value(source)?, range, shy))
}

/// The entity in register `r`, which the read empties.
#[inline(always)]
fn taken(registers: &mut [Option<Entity>], r: u32) -> Entity {
    registers[r as usize].take().expect("an instruction reads a register that an earlier one wrote")
}

/// Writes `entity` to register `to`, and gives whether its category differs from the recorded `check`.
#[inline(always)]
fn put(registers: &mut [Option<Entity>], to: u32, entity: Entity, check: Category) -> bool {
    let differs = entity.category() != check;
    registers[to as usize] = Some(entity);
    differs
}

/// `value` as a step reads it: with `call`, a function or operator atom, as a call's result holds one, is that function or operator (`Binding::from_element`). Any other value is unchanged.
#[inline(always)]
fn called_as(value: Binding, call: bool) -> Binding {
    match value { Binding::Value(v @ (Value::Function(_) | Value::Operator(_))) if call => Binding::from_element(v), value => value }
}

/// Fits the entity in register `to` to `category`, the one that the compiler fixed (`Entity::fit`).
#[cold]
#[inline(never)]
fn refit(registers: &mut [Option<Entity>], to: u32, category: Category) -> Result<(), Error> {
    if let Some(entity) = registers[to as usize].take() { registers[to as usize] = Some(entity.fit(category)?); }
    Ok(())
}

/// Fits what a call put in register `to` to `result`, the category that the compiler fixed. Only the statement's last call can give no result: registers are reused, so a register that matches the result's can hold an earlier call's.
#[cold]
#[inline(never)]
fn returned(registers: &mut [Option<Entity>], to: u32, result: Category, last: bool, span: &Span) -> Result<(), Error> {
    let entity = registers[to as usize].take().expect("a call's result");
    if !last && matches!(entity.term, Term::Binding(Binding::NoResult)) { return Err(Error::new(ErrorKind::Value, "expression produced no value").at(span)); }
    registers[to as usize] = Some(entity.fit(result)?);
    Ok(())
}

/// The expression that dissects the entity in register `read`, taken out of it, or `None` for a constant or an entity with none.
#[cold]
fn taken_expression(registers: &mut [Option<Entity>], read: Read) -> Option<Box<Expression>> {
    match read { Read::Register(r) => registers[r as usize].as_mut()?.expression.take(), Read::Constant(_) => None }
}

/// The expression that dissects a call of `f` on `y`, with `x` as its left argument where it has one: each argument's own expression, from `parts`, or the argument itself.
#[cold]
pub(super) fn dissected(f: &Function, x: Option<&Value>, y: &Value, [left, right]: [Option<Box<Expression>>; 2]) -> Box<Expression> {
    Box::new(Expression::call(f, x.map(|x| left.map_or_else(|| Expression::array(x), |e| *e)), right.map_or_else(|| Expression::array(y), |e| *e)))
}

/// The expression that dissects the call that a derive made of `f` on `y`, with `x` as its left argument where it has one. `operand`, the operand's own expression, stands for the argument that the operand gave.
#[cold]
fn derived_expression(f: &Function, x: Option<&Value>, y: &Value, operand: Option<Box<Expression>>) -> Box<Expression> {
    let operand = operand.map_or_else(|| Expression::array(x.unwrap_or(y)), |e| *e);
    Box::new(match x { Some(_) => Expression::call(f, Some(operand), Expression::array(y)), None => Expression::call(f, None, operand) })
}

/// The error for a name whose role changed while a statement that reads it ran, which the statement's compiled binding can't follow. A write that the statement's text doesn't show as a plain assignment changes a role then: a selective write in the statement, or a write inside `⍎`. A call's write that would change the role of a name outside its frame fails at the write instead (`Session::outer_role`).
#[cold]
fn role_changed(span: &Span) -> Error {
    Error::new(ErrorKind::Syntax, "a name changed between an array, a function and an operator while a statement that reads it ran").at(span)
}

/// What a step that pairs its two operands gives from the entities it reads: a bound right operand, a strand, a bound left argument or a train. The executor runs it on what its registers and constants hold, and the compiler on two constants, to fold them. With `capture`, a strand or a bound left argument keeps its parts' expressions.
pub(super) fn paired(op: &Op, mut left: Cow<'_, Entity>, right: Cow<'_, Entity>, source: &Arc<Source>, capture: bool) -> Result<Entity, Error> {
    Ok(match op {
        Op::BindRight { call, .. } => {
            let range = left.range.clone();
            let span = span_of(source, &range);
            Entity::of(bound_right(left.into_owned().value(source)?, called_as(right.into_owned().value(source)?, *call), &span)?, range, false)
        }
        Op::Strand { .. } => {
            let (mut left, mut right) = (left.into_owned(), right.into_owned());
            let parts = capture.then(|| (left.expression.take(), right.expression.take()));
            let (items, from_left, range) = strand(left, right, source)?;
            let expression = parts.map(|(l, r)| {
                let mut parts = Expression::items(l.map(|e| *e), &items[..from_left]);
                parts.extend(Expression::items(r.map(|e| *e), &items[from_left..]));
                Box::new(Expression::Strand(parts))
            });
            Entity::new(Term::Strand(items), range, false, expression)
        }
        Op::Attach { .. } => {
            let expression = match &mut left { Cow::Owned(entity) if capture => entity.expression.take(), _ => None };
            let (x, f, range) = attached(left, right, source)?;
            Entity::new(Term::Left(FunctionOperand::from(x.into_owned()), f.into_owned()), range, false, expression)
        }
        Op::Train { .. } => { let range = left.range.clone(); Entity::new(Term::Train(train(left, right, source)?), range, false, None) }
        _ => unreachable!("a step that pairs two operands"),
    })
}

/// The bracket literal `node`, built from `pieces`: its items as their levels prepared them (`StatementCode::prepared`) and its constant items, with each keyed item's entries.
#[inline]
pub(super) fn assembled<'e>(node: &Node, pieces: impl Iterator<Item = Cow<'e, Entity>>) -> Result<Binding, Error> {
    let NodeKind::ArrayLiteral { record, .. } = &node.kind else { unreachable!("a bracket literal") };
    if !*record {
        let mut arrays = Vec::with_capacity(pieces.size_hint().0);
        arrays.extend(pieces.filter_map(piece_value));
        return Session::list_literal(node, arrays);
    }
    let mut entries = Vec::with_capacity(pieces.size_hint().0);
    for piece in pieces {
        match piece {
            Cow::Owned(Entity { term: Term::Entries(more), .. }) => entries.extend(more),
            Cow::Borrowed(Entity { term: Term::Entries(more), .. }) => entries.extend_from_slice(more),
            piece => entries.extend(piece_value(piece).map(|value| (None, value))),
        }
    }
    Session::record_literal(node, entries)
}

/// The array that a piece of a bracket literal holds, cloned only where the piece is borrowed.
#[inline]
fn piece_value(piece: Cow<'_, Entity>) -> Option<Value> {
    match piece {
        Cow::Owned(Entity { term: Term::Binding(Binding::Value(value)), .. }) => Some(value),
        Cow::Borrowed(Entity { term: Term::Binding(Binding::Value(value)), .. }) => Some(value.clone()), _ => None
    }
}

/// What a bracket item in cell `cell` of the literal `literal` gives it, from the item's `value`: nothing for an absent `⍺`, the array (`session::listed`), or, with a key and the position of the item's colon, the item's entries (`Session::keyed`).
pub(super) fn piece(session: &Session, literal: &Node, cell: usize, value: Binding, key: Option<(&Value, usize)>, span: &Span) -> Result<Term, Error> {
    let Some(value) = super::session::listed(value, span)? else { return Ok(Term::Binding(Binding::Absent)) };
    Ok(match key { None => Term::Binding(Binding::Value(value)), Some((key, colon)) => Term::Entries(session.keyed(literal, cell, colon, key, &value)?) })
}

/// What an item of a compiled statement holds (`Entity`): a binding, or a term that the binder hasn't finished: a train's tines, an array bound as a function's left argument, a strand's items, a postfix, or a keyed item's entries.
#[derive(Clone, Debug)]
pub(super) enum Term {
    Binding(Binding),
    /// Functions and arrays side by side before a function with no argument.
    Train(Vec<Tine>),
    Left(FunctionOperand, Callable),
    /// Arrays side by side, one item each. They become a vector when used.
    Strand(Vec<Value>),
    /// A postfix, which compiles by its operand's role: the binder's term for it, never a value.
    Postfix(Postfix),
    /// A keyed bracket item's entries, which only its literal reads, in a compiled statement.
    Entries(Vec<crate::keyed::Entry>),
}
pub(super) type Tine = FunctionOperand;

pub(crate) fn before(value: FunctionOperand, function: Function, span: &Span) -> Result<Function, Error> {
    Function::train_operands(vec![value, FunctionOperand::from(Value::Function(function))], span)
}
pub(super) fn atop(first: Function, second: Function, span: &Span) -> Result<Function, Error> {
    Function::train_operands(vec![Value::Function(first).into(), Value::Function(second).into()], span)
}
pub(super) fn bound_right(operator: Binding, operand: Binding, span: &Span) -> Result<Binding, Error> {
    let Binding::Operator(OperatorValue::Operator(op)) = operator else { return Err(Error::domain("a right operand needs an unbound operator").at(span)) };
    Ok(Binding::Operator(OperatorValue::Bound { op, right: FunctionOperand::from(operand.into_value(span)?) }))
}
pub(super) fn derived(operand: Binding, operator: Binding, span: &Span) -> Result<Binding, Error> {
    let Binding::Operator(operator) = operator else { return Err(Error::domain("an operand needs an operator").at(span)) };
    operator.derive_operands(FunctionOperand::from(operand.into_value(span)?), None, span).map(Binding::from).map_err(|error| error.at(span))
}
pub(super) fn tails(function: &Callable) -> bool { function.definition().is_some() }
/// An item of a compiled statement, in a register or among its constants: its term, its text, whether it's shy, its category, and, while `Session::capture` is on, the expression that dissects it.
#[derive(Clone, Debug)]
pub(super) struct Entity {
    pub(super) term: Term,
    /// Source offsets; registers share the statement's Source.
    pub(super) range: std::ops::Range<u32>,
    pub(super) shy: bool,
    /// The entity's own category, from its term (`Category::bound`).
    category: Category,
    pub(super) expression: Option<Box<Expression>>,
}

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
    /// An entity with `term`.
    #[inline(always)]
    pub(super) fn new(term: Term, range: std::ops::Range<u32>, shy: bool, expression: Option<Box<Expression>>) -> Self {
        let category = match term {
            Term::Binding(ref value) => Category::bound(value), Term::Strand(_) | Term::Entries(_) => Category::Value, Term::Train(_) => Category::Train,
            Term::Left(..) => Category::Left, Term::Postfix(_) => Category::Operator
        };
        Self { term, range, shy, category, expression }
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
    /// What an unfinished term builds: a train's function, a bound left argument's function, a strand's vector, or the operator that a postfix stands for after a function (`Postfix::operator`).
    // Out of line, so that the conversions that every compiled operand goes through stay small enough to inline.
    #[inline(never)]
    fn built(self, source: &Arc<Source>) -> Result<Binding, Error> {
        let span = || span_of(source, &self.range);
        Ok(match self.term {
            Term::Binding(value) => value, Term::Train(tines) => Binding::from(Function::train_operands(tines, &span())?),
            Term::Postfix(postfix) => Binding::Operator(postfix.operator(&span())?), Term::Left(a, f) => Binding::from(before(a, f.function()?, &span())?),
            Term::Strand(items) => Binding::Value(Operation::join(Notation::Strand, Widening::Items).call(&items).map_err(|error| error.at(&span()))?),
            Term::Entries(_) => unreachable!("a keyed item's entries go only into its literal")
        })
    }
    #[inline(always)]
    pub(super) fn function(self, source: &Arc<Source>) -> Result<Callable, Error> {
        let range = self.range.clone();
        match self.value(source)? {
            Binding::Function(f) => Ok(f),
            value => {
                let function = Function::from_value(value, &span_of(source, &range))?;
                Ok(match session().bindings.inputs() {
                    Some(dependencies) => Callable::Rebuild { template: function, dependencies }, None => Callable::Input(function)
                })
            }
        }
    }
    #[inline(always)]
    pub(super) fn array(self, source: &Arc<Source>) -> Result<Value, Error> { match self.value(source)? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
    /// The entity with the category `category` that a compiled statement fixed for it, and a binding that fits it: a function or an operator that binds as an array becomes an atom, as a call's result does whatever it holds, and a function atom that binds as a function becomes the function again.
    pub(super) fn fit(self, category: Category) -> Result<Self, Error> {
        let term = match self.term {
            Term::Binding(Binding::Function(f)) if category == Category::Value => Term::Binding(Binding::Value(Value::Function(f.function()?))),
            Term::Binding(Binding::Operator(op)) if category == Category::Value => Term::Binding(Binding::Value(Value::Operator(Arc::new(op)))),
            Term::Binding(Binding::Value(v)) if category != Category::Value => Term::Binding(Binding::from_element(v)), term => term
        };
        Ok(Self { term, category, ..self })
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

/// `entity` as an argument: its array, which stays borrowed where a borrowed entity holds one, as a statement's constant does (`tree.rs`).
#[inline(always)]
pub(super) fn array_of<'p>(entity: Cow<'p, Entity>, source: &Arc<Source>) -> Result<Cow<'p, Value>, Error> {
    match entity {
        Cow::Borrowed(Entity { term: Term::Binding(Binding::Value(v)), .. }) => Ok(Cow::Borrowed(v)),
        entity => entity.into_owned().array(source).map(Cow::Owned),
    }
}

/// `entity` as a called function, which stays borrowed where a borrowed entity holds one.
#[inline(always)]
pub(super) fn function_of<'p>(entity: Cow<'p, Entity>, source: &Arc<Source>) -> Result<Cow<'p, Callable>, Error> {
    match entity {
        Cow::Borrowed(Entity { term: Term::Binding(Binding::Function(f)), .. }) => Ok(Cow::Borrowed(f)),
        entity => entity.into_owned().function(source).map(Cow::Owned),
    }
}

/// `left` bound as `right`'s left argument: `left` as an argument, then `right` as a function. The pair takes `right`'s text range, because a call belongs to its function's text.
#[inline]
pub(super) fn attached<'p>(
    left: Cow<'p, Entity>, right: Cow<'p, Entity>, source: &Arc<Source>) -> Result<(Cow<'p, Value>, Cow<'p, Callable>, std::ops::Range<u32>), Error> {
    let range = right.range.clone();
    let x = array_of(left, source)?;
    Ok((x, function_of(right, source)?, range))
}

/// A call's left argument and function, from the entity before its right argument: a bound left argument's parts, or no left argument and the entity as a function.
#[inline]
pub(super) fn split_call<'p>(left: Cow<'p, Entity>, source: &Arc<Source>) -> Result<(Option<Cow<'p, Value>>, Cow<'p, Callable>), Error> {
    Ok(match left {
        Cow::Owned(Entity { term: Term::Left(FunctionOperand::Value { value }, f), .. }) => (Some(Cow::Owned(value)), Cow::Owned(f)),
        Cow::Borrowed(Entity { term: Term::Left(FunctionOperand::Value { value }, f), .. }) => (Some(Cow::Borrowed(value)), Cow::Borrowed(f)),
        left => (None, function_of(left, source)?),
    })
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
        Cow::Owned(Entity { term: Term::Left(a, f), .. }) => tines.extend([a, FunctionOperand::from(Value::Function(f.function()?))]),
        Cow::Borrowed(Entity { term: Term::Left(a, f), .. }) => tines.extend([a.clone(), FunctionOperand::from(Value::Function(f.function()?))]),
        entity if matches!(entity.category, Category::Value | Category::Callable) => {
            tines.push(FunctionOperand::from(array_of(entity, source)?.into_owned()))
        }
        entity => tines.push(FunctionOperand::from(Value::Function(function_of(entity, source)?.function()?))),
    }
    Ok(())
}

/// The value of a leaf that the compiler folds (target part 18): a literal, a primitive or system function, or a primitive operator.
pub(super) fn constant(node: &Node) -> Binding {
    match &node.kind {
        NodeKind::Literal(a) => Binding::Value(a.clone()),
        NodeKind::Function(builtin) => Binding::from(builtin.function()),
        NodeKind::Operator(kind) => Binding::Operator(OperatorValue::Operator(Operator::Kind(kind.clone()))),
        NodeKind::BoundOperator(kind, right) => {
            Binding::Operator(OperatorValue::Bound { op: Operator::Kind(kind.clone()), right: FunctionOperand::from(Value::constant(right.clone())) })
        }
        _ => unreachable!("a literal, a function or an operator"),
    }
}

/// The dfn `definition` read where it's written, closing over the running frame and module: a function, or an operator where it names `⍶`.
fn closure(session: &Session, definition: &Arc<Definition>, _: &Span) -> Result<Binding, Error> {
    let function = Function::defined(definition.clone(), session.bindings.current(), session.bindings.module());
    Ok(match definition.kind {
        DefinitionKind::Function => Binding::from(function),
        DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Binding::Operator(OperatorValue::Operator(Operator::Closure(function))),
    })
}

/// The error for a read of `name` that finds no value, as `found` holds it: a local that a read already released (`Binding::Released`), `$e` outside a handler, or a name that nothing binds.
fn unbound(name: Name, found: Option<Binding>, span: &Span) -> Error {
    match found {
        Some(_) => released(name, span),
        None if name == Name::ERROR => Error::new(ErrorKind::Value, "$e is the error a handler caught, and exists only in a handler").at(span),
        None => undefined(name, span),
    }
}

/// `ᵘ` after the function `f`: a scalar that holds it.
fn unit(f: Function, span: &Span) -> Result<Value, Error> { Value::new(Vec::<usize>::new(), vec![Value::Function(f)]).described("invalid scalar").at(span) }

/// A postfix that compiles by its operand's role (target parts 18, 19, 21): a numeric subscript, `ᵀ`, `ᵘ`, a digit power (`⁻¹` included) or a mark. After an array it stands for a call (`call`); after a function, for an operator (`operator`), or for `ᵘ`'s scalar (`unit`).
#[derive(Clone, Debug)]
pub(super) enum Postfix { Index(Value), Transpose, Unit, Power(i64), Mark(crate::number::Mark) }

impl Postfix {
    fn parts(&self, a: Value) -> (Operation, Option<Value>, Value) {
        let primitive = |primitive| Operation::Primitive { primitive, pairing: crate::primitive::Pairing::Axes };
        match self {
            Self::Index(index) => (primitive(Primitive::Index), Some(index.clone()), a), Self::Transpose => (primitive(Primitive::Transpose(false)), None, a),
            Self::Power(n) => (primitive(Primitive::Math(crate::number::Math::Power)), Some(a), Value::constant(Value::weak_integer(*n))),
            Self::Mark(mark) => (mark.conversion(), None, a), Self::Unit => unreachable!("ᵘ after an array compiles to an error")
        }
    }
    pub(super) fn call(&self, a: Value, span: &Span) -> Result<Value, Error> {
        let (operation, x, y) = self.parts(a);
        let args: Vec<_> = x.into_iter().chain([y]).collect();
        operation.call(&args).map_err(|error| error.at(span))
    }
    fn expression(&self, a: &Value, operand: Option<Box<Expression>>) -> Option<Box<Expression>> {
        let (Operation::Primitive { primitive, .. }, x, y) = self.parts(a.clone()) else { return None };
        Some(derived_expression(&Function::primitive(primitive), x.as_ref(), &y, operand))
    }
    pub(super) fn operator(&self, span: &Span) -> Result<OperatorValue, Error> {
        let kind = |kind| OperatorValue::Operator(Operator::Kind(kind));
        Ok(match self {
            Self::Index(index) => OperatorValue::Bound { op: Operator::Kind(OperatorKind::Axis), right: FunctionOperand::from(index.clone()) },
            Self::Power(-1) => kind(OperatorKind::Inverse),
            Self::Power(n) => OperatorValue::Bound { op: Operator::Kind(OperatorKind::Power), right: FunctionOperand::from(Value::constant(Value::integer(*n))) },
            Self::Mark(mark) => kind(OperatorKind::Mark(*mark)), Self::Transpose => return Err(Error::domain("ᵀ transposes the array before it").at(span)),
            Self::Unit => return Err(Error::domain("ᵘ makes a scalar that holds the function before it").at(span))
        })
    }
    pub(super) fn of(operator: &OperatorValue) -> Option<Self> {
        match operator {
            OperatorValue::Bound { op: Operator::Kind(OperatorKind::Axis), right } => Some(Self::Index(right.stored()?.clone())),
            OperatorValue::Bound { op: Operator::Kind(OperatorKind::Power), right } => {
                let count = right.stored()?;
                count.data().as_number()?.integer().ok().map(Self::Power)
            }
            OperatorValue::Operator(Operator::Kind(OperatorKind::Inverse)) => Some(Self::Power(-1)),
            OperatorValue::Operator(Operator::Kind(OperatorKind::Mark(mark))) => Some(Self::Mark(*mark)), _ => None
        }
    }
}

/// The postfix of a letter superscript (`ᵀ`, `ᵘ`).
pub(super) fn lettered(letter: Letter) -> Postfix { match letter { Letter::Transpose => Postfix::Transpose, Letter::Unit => Postfix::Unit } }

/// What `postfix` gives on `operand`, for whose role the compiler compiled it: the call that it stands for on an array (`Postfix::call`), or `ᵘ`'s scalar holding a function (`unit`). The executor runs it, and the compiler runs it on a constant to fold it.
pub(super) fn postfixed(postfix: &Postfix, operand: Binding, span: &Span) -> Result<Binding, Error> {
    match (postfix, operand) {
        (Postfix::Unit, Binding::Function(f)) => Ok(Binding::Value(unit(f.function()?, span)?)),
        (postfix, Binding::Value(a)) => Ok(Binding::Value(postfix.call(a, span)?)), _ => unreachable!("the compiler compiles a postfix for its operand's role")
    }
}

impl StatementCode {
    /// Whether every fact that the statement's binding read still holds, which the statement checks as it starts (target part 18: roles are read again at each statement's start).
    #[inline(always)]
    fn holds(&self, session: &Session) -> Result<bool, Error> { for fact in &self.facts { if fact.changed(session)? { return Ok(false); } } Ok(true) }

    /// The statement evaluated by its compiled steps (`StatementCode::run_with`).
    #[inline(always)]
    pub(super) fn run(&self, session: &Session, tail: bool) -> Result<Step, Error> { self.run_with(session, tail, |_| ()) }

    /// The statement evaluated by its compiled steps, with `seed` in register 0, where the statement compiled with one (`binder::Compiler::compile`).
    pub(super) fn run_seeded(&self, session: &Session, seed: Entity) -> Result<Step, Error> {
        self.run_with(session, false, |registers| registers[0] = Some(seed))
    }

    /// The statement evaluated by its compiled steps, once `prepare` has set what registers hold at the start. It checks for an interrupt once, when it starts, and calls check for themselves. The run gets the statement's own slots, `registers` of them, inline up to `INLINE` and otherwise on the heap. Only those slots can hold anything when it ends. Emptying them and forgetting the inline array saves dropping all `INLINE` slots on every statement. Its end is a checkpoint (target, "Measures"): an error that forcing recorded in the session's slot raises there (`Session::raise`).
    #[inline(always)]
    fn run_with(&self, session: &Session, tail: bool, prepare: impl FnOnce(&mut [Option<Entity>])) -> Result<Step, Error> {
        if !self.holds(session)? { return Err(role_changed(&self.span)); }
        crate::execution::check().at(&self.span)?;
        let n = self.registers;
        let step = if n > INLINE { self.run_in(&mut std::iter::repeat_with(|| None).take(n).collect::<Vec<_>>(), session, tail, prepare) } else {
            let mut inline: [Option<Entity>; INLINE] = Default::default();
            let result = self.run_in(&mut inline[..n], session, tail, prepare);
            inline[..n].iter_mut().for_each(|slot| *slot = None);
            std::mem::forget(inline);
            result
        }?;
        session.raise()?;
        Ok(step)
    }

    /// `run_with`'s run on `registers`. One loop runs every step, so a level adds no interrupt check or stack growth. Each level still counts toward the depth limit while it runs, as `Session::nested` counts what the binder evaluates.
    #[inline(always)]
    fn run_in(&self, registers: &mut [Option<Entity>], session: &Session, tail: bool, prepare: impl FnOnce(&mut [Option<Entity>])) -> Result<Step, Error> {
        prepare(registers);
        let execute = |registers: &mut [Option<Entity>]| {
            if session.capture.get() { self.execute::<true>(session, tail, registers) } else { self.execute::<false>(session, tail, registers) }
        };
        if self.levels.is_empty() { return execute(registers); }
        // An error inside a level leaves the counts of its open levels behind, so the depth goes back to what it was.
        let depth = session.depth.get();
        let result = execute(registers);
        session.depth.set(depth);
        result
    }

    /// Runs the steps. A step whose value has another category than the compiler fixed fits it to that category (`refit`, `returned`), or a name's read does what its `Mismatch` says. With `CAPTURE`, which `run_with` sets while `Session::capture` is on, each call's result keeps the expression that dissects it (`Expression`). Every level of a recursion keeps this frame. In the browser, the stack that a level takes limits how deep recursion goes (`MAX_CALL_DEPTH`). Calls, the steps that recursion passes through, reads of names and the ends of levels run here. The other steps run out of line (`other_step`).
    fn execute<const CAPTURE: bool>(&self, session: &Session, tail: bool, registers: &mut [Option<Entity>]) -> Result<Step, Error> {
        let source = &self.span.source;
        for op in &self.ops {
            match op {
                Op::Load { name, address, take, to, range, check, on } => {
                    let taken = if *take { session.bindings.take_at(address) } else { None };
                    let found = match taken { Some(value) => Some(value), None => session.bindings.load(address)? };
                    let value = match found {
                        Some(value) if !matches!(value, Binding::Released) => value, found => return Err(unbound(*name, found, &span_of(source, range)))
                    };
                    if put(registers, *to, Entity::of(value, range.clone(), false), *check) {
                        match on { Mismatch::Fit => refit(registers, *to, *check)?, Mismatch::Fail => return Err(role_changed(&span_of(source, range))) }
                    }
                }
                Op::Enter { level } => {
                    // A level counts toward the depth limit until it exits, as `Session::nested` counts what it evaluates.
                    if !session.deeper() { return Err(depth_error(&self.levels[*level as usize - 1].span)); }
                }
                Op::Exit { level, from } => {
                    let held = &self.levels[*level as usize - 1];
                    let mut entity = self.owned(registers, *from);
                    if !matches!(entity.term, Term::Binding(_)) { entity = settled(entity, source)?; }
                    (entity.range, entity.shy) = (held.range.clone(), false);
                    registers[held.to as usize] = Some(if entity.category() == held.check { entity } else { entity.fit(held.check)? });
                    session.ascend();
                }
                Op::Call { left, function, argument, to, span, range, result, last } => {
                    let parts = CAPTURE.then(|| [taken_expression(registers, (*left).unwrap_or(*function)), taken_expression(registers, *argument)]);
                    let (x, f) = match left {
                        Some(x) => (Some(self.argument(registers, *x, source)?), self.called(registers, *function, source)?),
                        None => split_call(self.read(registers, *function), source)?,
                    };
                    let y = self.argument(registers, *argument, source)?;
                    let args: Vec<_> = x.iter().map(|x| x.as_ref().clone()).chain([y.as_ref().clone()]).collect();
                    let mut origin = f.origin(&args)?;
                    origin.site = Some(span.clone());
                    if tail && *last && tails(&f) { return Ok(Step::Tail(Application { origin, span: span.clone(), unshy: false })); }
                    let expression = match parts { Some(parts) => Some(dissected(&f.function()?, x.as_deref(), &y, parts)), None => None };
                    let bound = Bound::from_outputs(Operation::dispatch(origin.reborrow()).at(span)?, span)?;
                    if put(registers, *to, Entity::new(Term::Binding(bound.value), range.clone(), bound.shy, expression), *result) {
                        returned(registers, *to, *result, *last, span)?;
                    }
                }
                Op::Derive { operand, operator, to, span, range, result, call } => {
                    let expression = if CAPTURE { taken_expression(registers, *operand) } else { None };
                    let (operand, operator) = (called_as(self.value(registers, *operand, source)?, *call), self.value(registers, *operator, source)?);
                    let postfix = match (&operand, &operator) { (Binding::Value(_), Binding::Operator(op)) => Postfix::of(op), _ => None };
                    let entity = match (postfix, operand) {
                        (Some(postfix), Binding::Value(a)) => {
                            let expression = if CAPTURE { postfix.expression(&a, expression) } else { None };
                            Entity::new(Term::Binding(Binding::Value(postfix.call(a, span)?)), range.clone(), false, expression)
                        }
                        (_, operand) => Entity::of(derived(operand, operator, span)?, range.clone(), false),
                    };
                    if put(registers, *to, entity, *result) { returned(registers, *to, *result, false, span)?; }
                }
                op => self.other_step::<CAPTURE>(session, registers, source, op)?,
            }
        }
        let (shy, expression) = match self.result {
            Read::Register(r) => {
                registers[r as usize].as_mut().map_or((false, None), |entity| (entity.shy, if CAPTURE { entity.expression.take() } else { None }))
            }
            Read::Constant(_) => (false, None),
        };
        Ok(Step::Done(Bound { value: called_as(self.value(registers, self.result, source)?, self.call), shy, expression }))
    }

    /// A step of `execute` that recursion seldom passes through, run out of line so that its locals take no room in the frame that each level of a recursion keeps.
    #[inline(never)]
    fn other_step<const CAPTURE: bool>(&self, session: &Session, registers: &mut [Option<Entity>], source: &Arc<Source>, op: &Op) -> Result<(), Error> {
        match op {
            Op::Closure { definition, to, range, check } => {
                let entity = Entity::of(closure(session, definition, &span_of(source, range))?, range.clone(), false);
                if put(registers, *to, entity, *check) { refit(registers, *to, *check)?; }
            }
            Op::Scope { root, body, to, range } => {
                let span = span_of(source, range);
                let record = self.argument(registers, *root, source)?.into_owned();
                if !session.deeper() { return Err(depth_error(&span)); }
                let bound = session.scoped(&record, &span, |session| match session.statement_step(body, false)? {
                    Step::Done(bound) => Ok(bound), Step::Tail(_) => unreachable!("a scope's body isn't in tail position")
                });
                session.ascend();
                let bound = bound?;
                if put(registers, *to, Entity::new(Term::Binding(bound.value), range.clone(), bound.shy, None), Category::Value) {
                    refit(registers, *to, Category::Value)?;
                }
            }
            Op::Number { text, mark, empty, to, range, constant } => {
                let span = span_of(source, range);
                let text: &str = text.map_or_else(|| &source.text[span.range.clone()], |text| text);
                let numeric = Numeric::current();
                let number = || crate::number::literal(text, *mark, numeric).at(&span);
                let value = match constant {
                    _ if *empty => Value::empty([0], Value::Number(number()?)).map_err(|kind| Error::new(kind, "invalid empty array").at(&span))?,
                    Some(constant) => constant.get(numeric, || Ok(Value::Number(number()?)))?,
                    None => Value::Number(number()?),
                };
                registers[*to as usize] = Some(Entity::of(Binding::Value(value), range.clone(), false));
            }
            Op::Input { to, range } => {
                let span = span_of(source, range);
                let mut origin = Operation::Effect(crate::primitive::Act::Input).origin(&[])?;
                origin.site = Some(span.clone());
                let line = Operation::dispatch(origin.reborrow()).and_then(|outputs| outputs.assemble()).map_err(|error| error.at(&span))?;
                registers[*to as usize] = Some(Entity::of(Binding::Value(line), range.clone(), false));
            }
            Op::Postfix { postfix, operand, to, span, range, result } => {
                let expression = if CAPTURE { taken_expression(registers, *operand) } else { None };
                let operand = self.value(registers, *operand, source)?;
                let expression = match &operand { Binding::Value(a) if CAPTURE => postfix.expression(a, expression), _ => None };
                let entity = Entity::new(Term::Binding(postfixed(postfix, operand, span)?), range.clone(), false, expression);
                if put(registers, *to, entity, *result) { returned(registers, *to, *result, false, span)?; }
            }
            Op::Fail { error } => return Err(error.clone()),
            Op::Array { literal, pieces, to, range, check } => {
                if self.array(registers, literal, pieces, *to, range, *check)? { refit(registers, *to, *check)?; }
            }
            Op::ExitItem { level, from } => self.item_exit(session, registers, &self.levels[*level as usize - 1], *from, source)?,
            Op::Assign { value, local, target, span, modifier, to, range, call } => {
                // The value's category was checked where it was made, so it's neither absent nor no result.
                let value = value.map(|value| self.value(registers, value, source).map(|value| called_as(value, *call))).transpose()?;
                let value = match modifier {
                    None => session.assigned(target, value, *local, span)?,
                    Some(f) => session.modified(target, self.called(registers, *f, source)?.function()?, value)?,
                };
                let entity = Entity::of(value, range.clone(), true);
                registers[*to as usize] = Some(if *call && entity.category() != Category::Value { entity.fit(Category::Value)? } else { entity });
            }
            Op::BindRight { operator: left, operand: right, to, .. }
            | Op::Strand { left, right, to }
            | Op::Attach { left, right, to, .. }
            | Op::Train { left, right, to, .. } => {
                let entity = paired(op, self.read(registers, *left), self.read(registers, *right), source, CAPTURE)?;
                registers[*to as usize] = Some(entity);
            }
            _ => unreachable!("execute runs its other steps itself"),
        }
        Ok(())
    }

    /// Builds the bracket literal `node` from `pieces` (`assembled`), writes it to register `to` with the text `range`, and gives whether its category differs from `check`.
    #[inline(never)]
    fn array(&self, registers: &mut [Option<Entity>], node: &Node, pieces: &[Read], to: u32, range: &Range<u32>, check: Category) -> Result<bool, Error> {
        let value = assembled(node, pieces.iter().map(|&piece| self.read(registers, piece)))?;
        Ok(put(registers, to, Entity::of(value, range.clone(), false), check))
    }

    /// Ends the level of a bracket item, `held`, whose value `from` holds: the value goes into register `held.to` at once, prepared for its literal (`StatementCode::prepared`), so the items act and fail in order.
    #[inline(never)]
    fn item_exit(&self, session: &Session, registers: &mut [Option<Entity>], held: &Level, from: Read, source: &Arc<Source>) -> Result<(), Error> {
        let value = self.value(registers, from, source)?;
        let prepared = self.prepared(session, registers, held, value, source)?;
        registers[held.to as usize] = Some(prepared);
        session.ascend();
        Ok(())
    }

    /// A bracket item's piece of its literal (`piece`), made by the level that finishes the item (`Keyed`). `held` is the item's level. Where the key is computed, it's the level's own value, and the item's value is in a register.
    fn prepared(&self, session: &Session, registers: &mut [Option<Entity>], held: &Level, value: Binding, source: &Arc<Source>) -> Result<Entity, Error> {
        let Kind::Item { literal, cell, key } = &held.kind else { unreachable!("a bracket item's level") };
        let (span, cell) = (&held.span, *cell as usize);
        let term = match *key {
            None => piece(session, literal, cell, value, None, span)?,
            Some(Keyed::Key { constant, colon }) => {
                let Term::Binding(Binding::Value(key)) = &self.constants[constant as usize].term else { unreachable!("a key is an array") };
                piece(session, literal, cell, value, Some((key, colon as usize)), span)?
            }
            Some(Keyed::Value { value: item, colon }) => {
                let key = value.into_value(span)?;
                piece(session, literal, cell, self.value(registers, item, source)?, Some((&key, colon as usize)), span)?
            }
        };
        Ok(Entity::new(term, held.range.clone(), false, None))
    }

    /// The entity that `read` gives: a register's, which it takes, or a constant, which it borrows.
    #[inline(always)]
    fn read<'p>(&'p self, registers: &mut [Option<Entity>], read: Read) -> Cow<'p, Entity> {
        match read { Read::Register(r) => Cow::Owned(taken(registers, r)), Read::Constant(c) => Cow::Borrowed(&self.constants[c as usize]) }
    }

    /// `read`'s entity as an argument (`array_of`).
    #[inline(always)]
    fn argument<'p>(&'p self, registers: &mut [Option<Entity>], read: Read, source: &Arc<Source>) -> Result<Cow<'p, Value>, Error> {
        array_of(self.read(registers, read), source)
    }

    /// `read`'s entity as a called function (`function_of`).
    #[inline(always)]
    fn called<'p>(&'p self, registers: &mut [Option<Entity>], read: Read, source: &Arc<Source>) -> Result<Cow<'p, Callable>, Error> {
        function_of(self.read(registers, read), source)
    }

    /// The entity that `read` gives, owned: a register's, which the read empties, or a copy of a constant.
    #[inline(always)]
    fn owned(&self, registers: &mut [Option<Entity>], read: Read) -> Entity { self.read(registers, read).into_owned() }

    /// `read`'s entity's value (`Entity::value`), with only a constant's binding copied.
    #[inline(always)]
    fn value(&self, registers: &mut [Option<Entity>], read: Read, source: &Arc<Source>) -> Result<Binding, Error> {
        match read {
            Read::Register(r) => taken(registers, r).value(source),
            Read::Constant(c) => match &self.constants[c as usize] {
                Entity { term: Term::Binding(value), .. } => Ok(value.clone()), constant => constant.clone().value(source)
            },
        }
    }
}

impl Session {
    /// Compiles the current categories once and executes the statement.
    pub(super) fn statement_step(&self, nodes: &[Node], tail: bool) -> Result<Step, Error> {
        super::binder::Compiler::compile(nodes, self, None)?.run(self, tail)
    }
}
