//! The form rewriter: the one home of fast combinations, the combinations of functions that BPL computes by an algorithm of its own. Each rule is one row of `RULES`: a pattern written in BPL over `⍵`, with `⍺` in a dyadic one, beside the function without a glyph that computes it (`Special`), as `⍟1+⍵` beside `Log1p`. Any other name in a pattern captures an argument, which the replacement takes as its left argument, as `k` does in `k↑⍋⍵`.
//!
//! The rewriter reads what a function computes, and what an expression computes, as one tree of calls (`Form`). It replaces each subtree that a rule matches, inner subtrees first, with a call of the rule's function. `Function::new` rewrites a function's monadic and dyadic forms (`function`), and `dispatch` calls the results in the function's place. The binder rewrites each list of nodes before it walks it, after dot access and joins (`expression`). So `⍟1+x`, `⍟ 1+x`, `⍟(x+1)`, the train `⍟1+` and `⍟∘(1+)` all compute through `log1p`, from the first call on. An expression is read as far as its functions are constant and its arguments are arrays, as `Session::static_category` reads them, which keeps what it read about names in the recording's facts.
//!
//! A combination's function gives the combination's results. Where its algorithm doesn't suit the arguments, or a running level carries one, it calls the functions it combines (`combined`), which the level then sees. `•explain` reports each combination that its own algorithm computes.
use super::*;
use crate::primitive::{Folding, Order, Pick, Special};
use crate::syntax::{Node, NodeKind, cover};
use rustymath::events::Route;
use std::{cell::Cell, ops::Range, sync::OnceLock};
use super::tree::Facts;

/// Each fast combination apart from the selections, which `rules` adds: its pattern, and the function that computes it.
const RULES: &[(&str, Special)] = &[
    ("⍟1+⍵", Special::Log1p), ("¯1+*⍵", Special::Expm1),
    ("⍋⍋⍵", Special::Ranks(false)), ("⍋⍒⍵", Special::Ranks(true)),
    ("(≢⊢)⌸⍵", Special::GroupCounts), ("⍺(≢⊢)⌸⍵", Special::GroupCounts),
    ("⍺(+/⊢)⌸⍵", Special::GroupFolds(Folding::Plus, false)), ("⍺(+⌿⊢)⌸⍵", Special::GroupFolds(Folding::Plus, true)),
    ("⍺(⌈/⊢)⌸⍵", Special::GroupFolds(Folding::Max, false)), ("⍺(⌈⌿⊢)⌸⍵", Special::GroupFolds(Folding::Max, true)),
    ("⍺(⌊/⊢)⌸⍵", Special::GroupFolds(Folding::Min, false)), ("⍺(⌊⌿⊢)⌸⍵", Special::GroupFolds(Folding::Min, true)),
    ("⍺(×/⊢)⌸⍵", Special::GroupFolds(Folding::Times, false)), ("⍺(×⌿⊢)⌸⍵", Special::GroupFolds(Folding::Times, true)),
    ("-.×⍵", Special::Determinant), ("+.×⍵", Special::Permanent),
];

/// The pattern of each monadic rule whose function maps its argument, as `⍟1+⍵`.
pub(crate) fn mapping_patterns() -> impl Iterator<Item = &'static str> {
    RULES.iter().filter(|&&(text, s)| !text.contains('⍺') && Primitive::Special(s).facts(false).extension == crate::primitive::Extension::Pervasive).map(|&(text, _)| text)
}

/// Whether a rule matches some combination in expression `code`, read in `session`.
pub(crate) fn rewrites(session: &Session, code: &str) -> bool {
    let Ok(parsed) = crate::syntax::parse(crate::error::Source::new("<code>", code)).complete() else { return false };
    parsed.statements.iter().any(|s| matches!(expression(session, &s.nodes, &mut None), Cow::Owned(_)))
}

/// Where an expression's form lies: a list of nodes, by its number in `Reader::lists`, and the nodes it covers there.
type At = (usize, Range<usize>);

/// What a function or an expression computes, as a tree of calls, with where an expression's form lies.
#[derive(Clone, Debug)]
struct Form { kind: Kind, at: Option<At> }

#[derive(Clone, Debug)]
enum Kind {
    /// An argument of a function or a pattern: `⍵`, or `⍺` where `true`.
    Arg(bool),
    /// A pattern's capture, by its number.
    Hole(usize),
    /// A literal, or an array that a function holds as an operand.
    Value(Value),
    /// An array that an expression's name, which it holds, or group gives, which the form doesn't read into.
    Subject(Option<Name>),
    /// A call of a function on its left and right arguments.
    Call(Function, Option<Box<Form>>, Box<Form>),
}

impl Form {
    fn new(kind: Kind) -> Self { Self { kind, at: None } }
    /// The needs mask of the functions that this form calls, which a form needs to match a rule (`possible`).
    fn needs(&self) -> u64 {
        match &self.kind { Kind::Call(f, left, right) => f.node.needs | left.as_ref().map_or(0, |l| l.needs()) | right.needs(), _ => 0 }
    }
}

/// A rule, read from its row: its pattern, the bit of the primitive or operator that its pattern applies outermost (`root`), whether its pattern spans more than one call, the function that replaces a match, and the argument that the function takes on its left, if any: `⍺`, or the pattern's capture.
struct Rule { pattern: Form, root: u64, spans: bool, with: Special, left: Option<Kind> }

impl Rule {
    fn read(text: &str, with: Special) -> Self {
        let parsed = crate::syntax::parse(crate::error::Source::new("<rule>", text)).complete().expect("a rule parses");
        let nodes = &parsed.statements[0].nodes;
        let mut reader = Reader { session: session(), facts: None, lists: vec![nodes], holes: Vec::new() };
        let Some((pattern, 0)) = reader.read(0, nodes.len()) else { panic!("rule {text} doesn't read as calls on ⍵") };
        assert!(reader.holes.len() <= 2, "rule {text} captures at most two arguments");
        let left = if reads(&pattern, true) { Some(Kind::Arg(true)) } else if reader.holes.is_empty() { None } else { Some(Kind::Hole(0)) };
        let Kind::Call(f, l, r) = &pattern.kind else { panic!("rule {text} isn't a call") };
        let spans = matches!(r.kind, Kind::Call(..)) || l.as_deref().is_some_and(|l| matches!(l.kind, Kind::Call(..)));
        Self { root: root(f), spans, pattern, with, left }
    }
    /// What a match of `form` binds, where `form` matches.
    fn matched<'f>(&self, form: &'f Form) -> Option<Binds<'f>> {
        let mut binds = Binds::default();
        matches(&self.pattern, form, Some(&mut binds)).then_some(binds)
    }
    /// The call of the rule's function that replaces a match, on copies of the arguments that the match bound.
    fn replaced(&self, binds: Binds<'_>, at: Option<At>) -> Form {
        let left = match &self.left { Some(Kind::Arg(_)) => binds.args[1].cloned(), Some(Kind::Hole(i)) => binds.holes[*i].cloned(), _ => None };
        let right = binds.args[0].cloned().expect("a pattern reads ⍵");
        Form { kind: Kind::Call(Function::primitive(Primitive::Special(self.with)), left.map(Box::new), Box::new(right)), at }
    }
}

/// Whether `form` reads the argument `⍺`, where `left`, or `⍵`.
fn reads(form: &Form, left: bool) -> bool {
    match &form.kind { Kind::Arg(side) => *side == left, Kind::Call(_, l, r) => l.as_deref().is_some_and(|l| reads(l, left)) || reads(r, left), _ => false }
}

/// The rules, read once, in `all`: `RULES`, and a selection for each order and pick, from `k↑⍋⍵` to `k⊃>⍵`. `masks` holds each rule's needs mask, in the same order, and `keys` the `keys` of all of them and of the spanning ones. The first `spanning` rules have patterns that span more than one call, as `⍟1+⍵` and `k↑⍋⍵` do. A pattern of one call, as `-.×⍵`, is one function, which `Function::new` rewrites.
struct Rules { all: Vec<Rule>, masks: Vec<u64>, keys: [u64; 2], spanning: usize }

/// The rules, read once. `None` while they're being read, since reading a pattern builds functions, which `Function::new` would rewrite.
fn rules() -> Option<&'static Rules> {
    static READ: OnceLock<Rules> = OnceLock::new();
    thread_local! { static READING: Cell<bool> = const { Cell::new(false) }; }
    if let Some(rules) = READ.get() { return Some(rules); }
    if READING.get() { return None; }
    READING.set(true);
    let selections = [Order::Grade(false), Order::Grade(true), Order::Sort(false), Order::Sort(true)].into_iter()
        .flat_map(|o| [Pick::Take, Pick::First, Pick::Index, Pick::Mix].map(|p| (format!("{}{}{}⍵", if p == Pick::First { "" } else { "k" }, p.primitive().glyph(), o.primitive().glyph()), Special::Selected(o, p))));
    let mut all: Vec<Rule> = RULES.iter().map(|&(text, with)| Rule::read(text, with)).chain(selections.map(|(text, with)| Rule::read(&text, with))).collect();
    all.sort_by_key(|r| !r.spans);
    let (masks, spanning): (Vec<u64>, _) = (all.iter().map(|r| r.pattern.needs()).collect(), all.partition_point(|r| r.spans));
    let keys = [keys(&masks), keys(&masks[..spanning])];
    READING.set(false);
    Some(READ.get_or_init(|| Rules { all, masks, keys, spanning }))
}

/// Whether some rule's pattern needs only what the needs mask `held` holds, given the rules' `masks` and their `keys`.
fn possible(masks: &[u64], keys: u64, held: u64) -> bool { held & keys != 0 && masks.iter().any(|m| m & !held == 0) }

/// One bit of each of `masks`, the one that the fewest of them share, together. A form that matches a rule holds its rule's bit, so a needs mask that holds none of them matches no rule.
fn keys(masks: &[u64]) -> u64 {
    let shared = |b: u32| masks.iter().filter(|m| *m >> b & 1 == 1).count();
    masks.iter().fold(0, |keys, &m| keys | (0..64).filter(|&b| m >> b & 1 == 1).min_by_key(|&b| shared(b)).map_or(0, |b| 1 << b))
}

/// The parts of a matched form that a match binds to the pattern's arguments and captures. A pattern has at most two captures.
#[derive(Default)]
struct Binds<'f> { args: [Option<&'f Form>; 2], holes: [Option<&'f Form>; 2] }

/// Whether `form` matches `pattern`. With `binds`, each argument and capture of the pattern binds where the pattern first reads it, and compares with that binding where it reads it again. Without, the two forms must compute the same thing: the same argument, number or name, or the same calls of them.
fn matches<'f>(pattern: &Form, form: &'f Form, mut binds: Option<&mut Binds<'f>>) -> bool {
    if let Some(b) = binds.as_deref_mut() {
        match pattern.kind {
            Kind::Arg(side) => return bind(&mut b.args[usize::from(side)], form),
            Kind::Hole(i) => return bind(&mut b.holes[i], form),
            _ => {}
        }
    }
    match (&pattern.kind, &form.kind) {
        (Kind::Arg(x), Kind::Arg(y)) => x == y, (Kind::Value(x), Kind::Value(y)) => same_value(x, y), (Kind::Subject(Some(x)), Kind::Subject(Some(y))) => x == y,
        (Kind::Call(g, pl, pr), Kind::Call(h, fl, fr)) => {
            same(g, h) && match (pl, fl) { (None, None) => true, (Some(p), Some(f)) => matches(p, f, binds.as_deref_mut()), _ => false } && matches(pr, fr, binds)
        }
        _ => false,
    }
}

fn bind<'f>(slot: &mut Option<&'f Form>, form: &'f Form) -> bool {
    match slot { Some(bound) => matches(bound, form, None), None => { *slot = Some(form); true } }
}

/// Whether two functions are built from the same parts, with the same whole numbers in them.
fn same(f: &Function, g: &Function) -> bool { f.equal_by(g, &mut |x, y, _| Ok::<_, Infallible>(same_value(x, y))).unwrap_or(false) }

fn same_value(x: &Value, y: &Value) -> bool { integer(x).is_some_and(|n| integer(y) == Some(n)) }

fn integer(v: &Value) -> Option<i64> { v.as_number()?.integer().ok() }

/// The call of `f` on `left` and `right`, with a whole-number right argument of `+`, `×` or `-` put first where the left one isn't a whole number, as patterns spell it: `x+1` reads as `1+x`, and `x-1` as `¯1+x`.
fn call(f: &Function, left: Option<Form>, right: Form, at: Option<At>) -> Form {
    use crate::number::Arithmetic::*;
    let whole = |form: &Form| match &form.kind { Kind::Value(v) => integer(v), _ => None };
    let constant = whole(&right).is_some() && left.as_ref().is_some_and(|l| whole(l).is_none());
    let negated = whole(&right).map(|n| Value::integer(-n));
    let kind = match (f.as_primitive(), left, negated) {
        (Some(Primitive::Arithmetic(Plus | Times)), Some(l), _) if constant => Kind::Call(f.clone(), Some(Box::new(right)), Box::new(l)),
        (Some(Primitive::Arithmetic(Minus)), Some(l), Some(n)) if constant => {
            Kind::Call(Function::primitive(Primitive::Arithmetic(Plus)), Some(Box::new(Form { kind: Kind::Value(n), at: right.at })), Box::new(l))
        }
        (_, left, _) => Kind::Call(f.clone(), left.map(Box::new), Box::new(right)),
    };
    Form { kind, at }
}

/// What `f` computes from the forms of its arguments, `left` absent in a monadic call, through its trains and compositions. Any other function is one call of itself.
fn applied(f: &Function, left: Option<Form>, right: Form) -> Form {
    use {FunctionNode as N, OperatorKind as O, Operand::{Function as F, Value as V}};
    match (f.node(), left) {
        (N::Composed(O::Atop, [F(g), F(h)]), left) => applied(g, None, applied(h, left, right)),
        (N::Composed(O::Before, [V(a), F(g)]), None) => applied(g, Some(Form::new(Kind::Value(a.clone()))), right),
        (N::Composed(O::Before, [F(g), F(h)]), left) => {
            let x = left.unwrap_or_else(|| right.clone());
            applied(h, Some(applied(g, None, x)), right)
        }
        (N::Composed(O::After, [F(g), V(a)]), None) => applied(g, Some(right), Form::new(Kind::Value(a.clone()))),
        (N::Composed(O::After, [F(g), F(h)]), left) => {
            let x = left.unwrap_or_else(|| right.clone());
            applied(g, Some(x), applied(h, None, right))
        }
        (N::Fork([g, h, k]), left) => applied(h, Some(applied(g, left.clone(), right.clone())), applied(k, left, right)),
        (N::Modified(O::Commute, F(g)), left) => {
            let x = left.unwrap_or_else(|| right.clone());
            applied(g, Some(right), x)
        }
        (_, left) => call(f, left, right, None),
    }
}

/// The function that computes `form` from its arguments: `⍺` and `⍵` where `dyadic`, and `⍵` alone otherwise.
fn built(form: &Form, dyadic: bool, span: &Span) -> Result<Function, Error> {
    use {FunctionNode as N, OperatorKind as O};
    let atop = |f: Function, g: Function| Function::new(N::Composed(O::Atop, [Operand::Function(f), Operand::Function(g)]), span);
    match &form.kind {
        Kind::Arg(left) => Ok(Function::primitive(Primitive::Identity(*left))),
        Kind::Value(a) => Function::constant(a.clone(), span),
        Kind::Call(f, None, r) if !dyadic && matches!(r.kind, Kind::Arg(false)) => Ok(f.clone()),
        Kind::Call(f, Some(l), r) if dyadic && matches!((&l.kind, &r.kind), (Kind::Arg(true), Kind::Arg(false))) => Ok(f.clone()),
        Kind::Call(f, None, r) => atop(f.clone(), built(r, dyadic, span)?),
        Kind::Call(f, Some(l), r) => match (&l.kind, dyadic) {
            (Kind::Value(a), false) => {
                let bound = Function::new(N::Composed(O::Before, [Operand::Value(a.clone()), Operand::Function(f.clone())]), span)?;
                if matches!(r.kind, Kind::Arg(false)) { Ok(bound) } else { atop(bound, built(r, dyadic, span)?) }
            }
            _ => Function::new(N::Fork([built(l, dyadic, span)?, f.clone(), built(r, dyadic, span)?]), span),
        },
        Kind::Hole(_) | Kind::Subject(_) => unreachable!("a function's form reads only its arguments"),
    }
}

/// The needs mask of a function with node `node`, from its own operator and its operands' masks, which `Function::new` keeps.
pub(super) fn built_from(node: &FunctionNode) -> u64 {
    let operand = |o: &Operand| if let Operand::Function(f) = o { f.node.needs } else { 0 };
    match node {
        FunctionNode::Primitive(p) => glyph(*p),
        FunctionNode::Fold(f, kind) => bit(OperatorKind::Fold(*kind).glyph()) | f.node.needs,
        FunctionNode::Modified(op, a) => bit(op.glyph()) | operand(a),
        FunctionNode::Composed(op, [a, b]) => bit(op.glyph()) | operand(a) | operand(b),
        FunctionNode::Fork(fs) => fs.iter().fold(0, |m, f| m | f.node.needs),
        _ => 0,
    }
}

/// The bit of the primitive or operator that the function `f` applies outermost, or 0 for any other function.
fn root(f: &Function) -> u64 {
    match f.node() {
        FunctionNode::Primitive(p) => bit(p.glyph()),
        FunctionNode::Fold(_, kind) => bit(OperatorKind::Fold(*kind).glyph()),
        FunctionNode::Modified(op, _) | FunctionNode::Composed(op, _) => bit(op.glyph()),
        _ => 0,
    }
}

/// The needs mask of the primitives and operators that `node` writes, in its groups too.
fn written(node: &Node) -> u64 {
    match &node.kind {
        NodeKind::Function(crate::primitive::Builtin::Primitive(p)) => glyph(*p),
        NodeKind::Operator(op) => bit(op.glyph()),
        NodeKind::Group(inner) | NodeKind::Run(inner) => inner.iter().fold(0, |m, n| m | written(n)),
        _ => 0,
    }
}

/// `form` with each subtree that a rule matches replaced by a call of the rule's function, inner subtrees first, with its needs mask. A subtree tries only the rules whose root it applies outermost and whose mask its own covers. `changed` records whether any subtree was replaced.
fn rewritten(form: Form, rules: &Rules, changed: &mut bool) -> (Form, u64) {
    let Form { kind: Kind::Call(f, left, right), at } = form else { return (form, 0) };
    let root = root(&f);
    let (left, held) = match left { Some(l) => { let (l, m) = rewritten(*l, rules, changed); (Some(Box::new(l)), m) } None => (None, 0) };
    let (right, mask) = rewritten(*right, rules, changed);
    let held = held | mask | f.node.needs;
    let form = Form { kind: Kind::Call(f, left, Box::new(right)), at };
    let mut tried = rules.all.iter().zip(&rules.masks).filter(|&(rule, mask)| rule.root == root && mask & !held == 0);
    match tried.find_map(|(rule, _)| Some((rule, rule.matched(&form)?))) {
        Some((rule, binds)) => { *changed = true; let replaced = rule.replaced(binds, form.at.clone()); let mask = replaced.needs(); (replaced, mask) }
        None => (form, held),
    }
}

/// The functions that compute `f`'s monadic and dyadic calls, where a rule matches part of what `f` computes in that call.
pub(super) fn function(f: &Function, span: &Span) -> [Option<Function>; 2] {
    let Some(rules) = rules() else { return [None, None] };
    if !possible(&rules.masks, rules.keys[0], f.node.needs) { return [None, None]; }
    [false, true].map(|dyadic| {
        let mut changed = false;
        let (form, _) = rewritten(applied(f, dyadic.then(|| Form::new(Kind::Arg(true))), Form::new(Kind::Arg(false))), rules, &mut changed);
        if changed { built(&form, dyadic, span).ok() } else { None }
    })
}

/// The structural combination that `f` computes and the value it binds on its left, where `built` made `f` as the combination's function, alone or with a value bound on its left. `dispatch` calls `combined` with them.
pub(super) fn direct(f: &Function) -> Option<(Special, Option<&Value>)> {
    let special = |g: &Function| match g.node() { FunctionNode::Primitive(Primitive::Special(s)) if s.combines() => Some(*s), _ => None };
    match f.node() {
        FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(g)]) => Some((special(g)?, Some(a))),
        _ => Some((special(f)?, None)),
    }
}

/// Whether a train with `tines` may hold a combination that a rule matches, so that the caller builds the train's function, which `function` rewrites, instead of calling its tines (`operators::calls_tines`).
pub(super) fn may_combine(tines: &[Tine]) -> bool {
    let Some(rules) = rules() else { return false };
    possible(&rules.masks, rules.keys[0], tines.iter().fold(0, |m, t| match t { Tine::Function(f) => m | f.node.needs, _ => m }))
}

/// `nodes` with each combination in them that a rule whose pattern spans more than one call matches (`Rules::spanning`) replaced by a group that calls the rule's function, or `nodes` as they are. Only a combination that lies in `nodes` itself is replaced here: one inside a group is replaced when the binder walks the group. `facts` keeps what a reading that found a combination read about what names hold.
pub(super) fn expression<'a>(session: &Session, nodes: &'a [Node], facts: &mut Facts) -> Cow<'a, [Node]> {
    let Some(Rules { all, masks, keys, spanning }) = rules() else { return Cow::Borrowed(nodes) };
    let rules = &all[..*spanning];
    if !possible(&masks[..*spanning], keys[1], nodes.iter().fold(0, |m, n| m | written(n))) { return Cow::Borrowed(nodes); }
    let mut out = Cow::Borrowed(nodes);
    loop {
        let mut read = Vec::new();
        let found = {
            let mut logged: Facts = facts.is_some().then_some(&mut read);
            let mut reader = Reader { session, facts: Some(&mut logged), lists: vec![&out[..]], holes: Vec::new() };
            reader.read(0, out.len()).and_then(|(form, _)| reader.first(&form, rules))
        };
        let Some((range, node)) = found else { return out };
        if let Some(facts) = facts { read.into_iter().for_each(|f| if !facts.contains(&f) { facts.push(f) }) }
        out.to_mut().splice(range, [node]);
    }
}

/// Reads forms from lists of nodes, right to left, as the binder groups them. A pattern's reader, with `facts` of `None`, reads `⍵` and `⍺` as arguments and any other name as a capture.
struct Reader<'a, 'f, 'r> { session: &'a Session, facts: Option<&'f mut Facts<'r>>, lists: Vec<&'a [Node]>, holes: Vec<Name> }

impl<'a> Reader<'a, '_, '_> {
    fn category(&mut self, node: &Node) -> Category {
        match (&mut self.facts, &node.kind) {
            (None, NodeKind::Name(..)) => Category::Value,
            (Some(facts), _) => self.session.static_category(node, facts),
            (None, _) => self.session.static_category(node, &mut None),
        }
    }

    /// The form of the longest run of calls that ends at `end` in list `list` and starts from an array, with where it starts.
    fn read(&mut self, list: usize, end: usize) -> Option<(Form, usize)> {
        let mut start = end.checked_sub(1)?;
        let mut form = self.subject(list, start)?;
        while let Some((f, from)) = self.function(list, start) {
            // An array just before the function is its left argument, unless an array before that strands with it.
            let nodes = self.lists[list];
            let (left, from) = match from.checked_sub(1).and_then(|i| Some((self.subject(list, i)?, i))) {
                Some((_, i)) if i > 0 && self.category(&nodes[i - 1]) == Category::Value => break,
                Some((left, i)) => (Some(left), i),
                None => (None, from),
            };
            form = call(&f, left, form, Some((list, from..end)));
            start = from;
        }
        Some((form, start))
    }

    /// The form of the array at node `i` of list `list`: a literal's value, a pattern's argument or capture, the calls in a group, or an array that a name or group gives. `None` where the node isn't an array, or is a dyadic operator's operand.
    fn subject(&mut self, list: usize, i: usize) -> Option<Form> {
        let nodes = self.lists[list];
        if i > 0 && self.category(&nodes[i - 1]).valence() == Some(true) { return None; }
        let kind = match &nodes[i].kind {
            NodeKind::Literal(v) => Kind::Value(v.clone()),
            NodeKind::Name(name, _) if self.facts.is_none() => match *name {
                Name::OMEGA => Kind::Arg(false), Name::ALPHA => Kind::Arg(true),
                _ => Kind::Hole(self.holes.iter().position(|h| h == name).unwrap_or_else(|| { self.holes.push(*name); self.holes.len() - 1 })),
            },
            NodeKind::Group(inner) | NodeKind::Run(inner) if self.category(&nodes[i]) == Category::Value => {
                self.lists.push(inner);
                match self.read(self.lists.len() - 1, inner.len()) { Some((form, 0)) => return Some(form), _ => Kind::Subject(None) }
            }
            NodeKind::Name(name, _) if self.category(&nodes[i]) == Category::Value => Kind::Subject(Some(*name)),
            _ if self.category(&nodes[i]) == Category::Value => Kind::Subject(None),
            _ => return None,
        };
        Some(Form { kind, at: Some((list, i..i + 1)) })
    }

    /// The constant function whose nodes end just before node `end` of list `list`, with where they start: a primitive, or operators on constant operands, as `+/`, `(≢⊢)⌸` and `-.×`.
    fn function(&mut self, list: usize, end: usize) -> Option<(Function, usize)> {
        let nodes = self.lists[list];
        let from = self.term(nodes, end.checked_sub(1)?)?;
        // No dyadic operator before the function may take it as its operand.
        if from > 0 && self.category(&nodes[from - 1]).valence() == Some(true) || !nodes[from..end].iter().all(constant) { return None; }
        let f = match &nodes[from..end] {
            [Node { kind: NodeKind::Function(crate::primitive::Builtin::Primitive(p)), .. }] => Function::primitive(*p),
            run => match Binder::evaluate(run, self.session, false).ok()? { Step::Done(Bound { value: Binding::Function(f), .. }) => f, _ => return None },
        };
        Some((f, from))
    }

    /// Where the function or operand whose last node is node `k` starts: a monadic operator takes the operand before it, and a dyadic operator the operands on both sides of it.
    fn term(&mut self, nodes: &[Node], k: usize) -> Option<usize> {
        match self.category(&nodes[k]).valence() {
            Some(false) => self.operand(nodes, k.checked_sub(1)?),
            Some(true) => None,
            None if k > 0 && self.category(&nodes[k - 1]).valence() == Some(true) => self.operand(nodes, k.checked_sub(2)?),
            None => (self.category(&nodes[k]) == Category::Function).then_some(k),
        }
    }

    /// Where the operand whose last node is node `k` starts: a function, or an array.
    fn operand(&mut self, nodes: &[Node], k: usize) -> Option<usize> {
        self.term(nodes, k).or_else(|| (self.category(&nodes[k]) == Category::Value).then_some(k))
    }

    /// The first subtree of `form`, inner subtrees first, that lies in the reader's first list and that a rule matches: the range of its nodes, and the group that replaces them.
    fn first(&self, form: &Form, rules: &[Rule]) -> Option<(Range<usize>, Node)> {
        let Kind::Call(_, left, right) = &form.kind else { return None };
        if let Some(found) = left.as_deref().and_then(|l| self.first(l, rules)).or_else(|| self.first(right, rules)) { return Some(found); }
        let Some((0, range)) = form.at.clone() else { return None };
        let (rule, binds) = rules.iter().find_map(|rule| Some((rule, rule.matched(form)?)))?;
        let Kind::Call(f, left, right) = rule.replaced(binds, None).kind else { unreachable!("a replacement is a call") };
        let span = cover(&self.lists[0][range.clone()]);
        let mut group: Vec<Node> = left.map(|l| self.node(&l)).into_iter().collect();
        group.push(Node { kind: NodeKind::Function(crate::primitive::Builtin::Primitive(f.as_primitive()?)), span: span.clone() });
        group.push(self.node(&right));
        Some((range, Node { kind: NodeKind::Group(group), span }))
    }

    /// The node that gives a matched argument: its own node, or a group of the nodes it covers.
    fn node(&self, form: &Form) -> Node {
        let (list, range) = form.at.clone().expect("an expression's forms know their nodes");
        match &self.lists[list][range] { [node] => node.clone(), nodes => Node { kind: NodeKind::Group(nodes.to_vec()), span: cover(nodes) } }
    }
}

/// Whether a node is constant: a primitive, an operator, a literal, or a group of those.
fn constant(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Function(crate::primitive::Builtin::Primitive(_)) | NodeKind::Operator(_) | NodeKind::Literal(_) => true,
        NodeKind::Group(inner) | NodeKind::Run(inner) => inner.iter().all(constant),
        _ => false,
    }
}

/// The Key or product that combination `s` replaces, which a running level sees in its place.
fn original(s: Special, span: &Span) -> Result<Function, Error> {
    use {crate::number::Arithmetic::*, FunctionNode as N, OperatorKind as O};
    let p = Function::primitive;
    let key = |f: Function| {
        let operand = Function::new(N::Composed(O::Atop, [Operand::Function(f), Operand::Function(p(Primitive::Identity(false)))]), span)?;
        Function::new(N::Modified(O::Key, Operand::Function(operand)), span)
    };
    let product = |f| Function::new(N::Composed(O::Product, [Operand::Function(p(Primitive::Arithmetic(f))), Operand::Function(p(Primitive::Arithmetic(Times)))]), span);
    match s {
        Special::GroupCounts => key(p(Primitive::Tally)),
        Special::GroupFolds(by, first) => key(Function::new(N::Fold(p(by.primitive()), FoldKind { scan: false, first }), span)?),
        Special::Determinant => product(Minus),
        Special::Permanent => product(Plus),
        _ => unreachable!("only Key's and the products' combinations have a function of their own to stand for"),
    }
}

/// A combination's function on these arguments, apart from `Log1p` and `Expm1`, which are kernels: its own algorithm where the arguments suit it, and otherwise the functions it combines. `call_axis` calls it before any running level, which then sees those functions.
pub(crate) fn combined(s: Special, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    use crate::number::Arithmetic::*;
    // A running level sees the Key or product that the combination replaces, and applies its own rule to it, as `∂` does to `+.×`.
    if matches!(s, Special::GroupCounts | Special::GroupFolds(..) | Special::Determinant | Special::Permanent) && crate::transform::carried(left, right) {
        return original(s, span)?.call_array(left, right, span);
    }
    let (route, fast) = match s {
        Special::Selected(order, pick) => (if matches!(order, Order::Grade(_)) { Route::PartialGrade } else { Route::PartialSort }, selected(order, pick, left, right, span)),
        Special::Ranks(down) => (Route::Ranks, ranks(down, right, span)),
        Special::GroupCounts | Special::GroupFolds(..) => return grouped(s, left, right, span)?.array(span),
        Special::Determinant => return primitive_expansion(Primitive::Arithmetic(Minus), Primitive::Arithmetic(Times), right, span),
        Special::Permanent => return primitive_expansion(Primitive::Arithmetic(Plus), Primitive::Arithmetic(Times), right, span),
        _ => unreachable!("a kernel computes this combination"),
    };
    if let Some(result) = fast {
        if result.is_ok() { taken(route, s) }
        return result;
    }
    rustymath::events::declined(route, "arguments that don't suit it", right.len());
    match s {
        Special::Selected(order, pick) => pick.primitive().call(left, &order.primitive().call(None, right, span)?, span),
        Special::Ranks(down) => Primitive::Grade(false).call(None, &Primitive::Grade(down).call(None, right, span)?, span),
        _ => unreachable!("only selections and ranks decline here"),
    }
}

/// Records for `•explain`, where it's on, that the function of combination `s` computed it by its own algorithm, along `route`. It names the combination by its row's glyph, as `↑⍋` names every selection.
pub(super) fn taken(route: Route, s: Special) {
    if rustymath::events::running() { crate::explain::fast(route, Primitive::Special(s).glyph().into()) }
}

/// The items of `right`'s order that `pick` takes, through rustymath's partial selection, where `right` is a vector without keys and they are at most a quarter of its items. `left` is the count to take or the position to pick.
fn selected(order: Order, pick: Pick, left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<Value, Error>> {
    if right.shape().len() != 1 || right.keys(0).is_some() || crate::transform::carried(left, right) { return None; }
    let n = right.len();
    let (range, item) = match pick {
        Pick::Take => {
            let k = integer(left?)?;
            let m = usize::try_from(k.unsigned_abs()).ok().filter(|&m| m <= n)?;
            (if k < 0 { n - m..n } else { 0..m }, false)
        }
        Pick::First => { let i = rustymath::layout::position(0, n)?; (i..i + 1, true) }
        Pick::Index | Pick::Mix => { let i = rustymath::layout::position(integer(left?)?, n)?; (i..i + 1, true) }
    };
    // Large runs cost more to comparison-sort than grading the whole vector by radix or counting.
    if range.len() > n / 4 { return None; }
    let (grade, down) = match order { Order::Grade(down) => (true, down), Order::Sort(down) => (false, down) };
    let taken = rustymath::search::view_selected(right.view()?, range, down)?;
    Some((|| {
        let positions = Value::positions_u32([taken.len()], n, taken).and_then(|p| p.along_first_of(right)).described("invalid selected order")?;
        let selected = if grade { positions } else {
            let selector = Value::new([1], vec![positions]).described("invalid selected order")?;
            Primitive::Index.call(Some(&selector), right, span)?
        };
        if item { crate::primitive::pick(&Value::integer(0), &selected, session().prototype.get(), None, span) } else { Ok(selected) }
    })())
}

/// Each major cell's rank in the order that `⍋` or, where `down`, `⍒` gives: the inverse of the grade's permutation.
fn ranks(down: bool, right: &Value, span: &Span) -> Option<Result<Value, Error>> {
    use {crate::array::Items, rustymath::buffer::View};
    if right.shape().is_empty() || right.keys(0).is_some() || crate::transform::carried(None, right) { return None; }
    let grade = match Primitive::Grade(down).call(None, right, span) { Ok(g) => g, Err(e) => return Some(Err(e)) };
    let Items::Compact(View::Integers(ints)) = grade.as_items() else { return Some(Primitive::Grade(false).call(None, &grade, span)) };
    let n = grade.len();
    let ranks = rustymath::with_ints!(ints, |positions| rustymath::search::inverse_permutation(positions.iter().map(|&p| i64::from(p) as usize)));
    Some(Value::positions_u32([n], n, ranks).and_then(|r| r.along_first_of(right)).described("invalid ranks"))
}

/// The bit that a primitive or operator with glyph `glyph` sets in a needs mask: its first character's position among the characters that the rules' patterns write, or none for a character that no pattern writes, which can't help a form match. The characters come from the rules' text, so a function's mask doesn't wait for the rules to be read.
fn bit(glyph: &str) -> u64 {
    static WRITTEN: OnceLock<Vec<char>> = OnceLock::new();
    let written = WRITTEN.get_or_init(|| {
        let selections = [Pick::Take, Pick::Index, Pick::Mix].map(|p| p.primitive().glyph()).into_iter()
            .chain([Order::Grade(false), Order::Grade(true), Order::Sort(false), Order::Sort(true)].map(|o| o.primitive().glyph()));
        let mut chars: Vec<char> = RULES.iter().map(|&(text, _)| text).chain(selections).flat_map(str::chars)
            .filter(|c| !c.is_alphanumeric() && !"⍺⍵¯() ".contains(*c)).collect();
        chars.sort_unstable();
        chars.dedup();
        assert!(chars.len() <= 64, "a needs mask holds 64 characters");
        chars
    });
    glyph.chars().next().and_then(|c| written.binary_search(&c).ok()).map_or(0, |i| 1 << i)
}

/// The needs mask of primitive `p`, which `Function::plain` keeps. A `-` also counts as `+`, as `call` reads `x-1` as `¯1+x`.
pub(super) fn glyph(p: Primitive) -> u64 {
    use crate::number::Arithmetic::*;
    bit(p.glyph()) | if p == Primitive::Arithmetic(Minus) { bit(Primitive::Arithmetic(Plus).glyph()) } else { 0 }
}
