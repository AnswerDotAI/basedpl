use rustymath::{buffer::FloatWidth, error::Kind};
use crate::{number::{subscript, width_marker, Arithmetic, Math, Numeric, Suffix, DENOMINATOR, EXACT, EXPONENT, IMAGINARY, SUFFIXES, WIDTHS}, primitive::{Builtin, OperatorKind, Primitive, Superscript}, symbols::canonical,
    Error, ErrorAt, ErrorKind, Number, Source, Span, Value
};
use crate::name::{is_implicit, Name};
use std::{iter::Peekable, str::CharIndices, sync::{atomic::{AtomicU64, Ordering::Relaxed}, Arc}};

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Literal(Value),
    Function(Builtin),
    Operator(OperatorKind),
    Name(Name),
    /// A system name that the system table doesn't hold, which is a VALUE error when evaluated. A known system name is a `Function`, as a glyph is.
    System(String),
    Assign,
    Output,
    ErrorGuard,
    Group(Vec<Node>),
    /// A run of nodes with no spaces between them, evaluated before its neighbours.
    Run(Vec<Node>),
    /// A subscript, as in `v₁`: `(1⌷v)` for the item just before it, which binding finds after dot access.
    Subscript(i64),
    /// `ⱼ`, `ₑ` or `ᵣ` between two items, as in `xⱼy`, which binding joins into one term once dot access and subscripts are bound.
    Joiner(Joiner),
    /// `T.(expr)`, which dot access builds: the second node evaluated with the keys of the first as names.
    Scope(Box<Node>, Box<Node>),
    /// An array literal. `record`: at least one item is `key:value`, and the items build one keyed vector.
    ArrayLiteral { cells: Vec<Vec<Node>>, form: ListForm, record: bool },
    Dfn(Arc<Definition>),
}

/// How an array literal is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListForm {
    /// `[a b]`: each cell is one item.
    Items,
    /// `[1 2⋄3 4]`: each cell is a major cell of the result.
    Cells,
    /// `(4⋄4 5)`: each cell is a row, and each row is one item.
    Rows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefinitionKind { Function, MonadicOperator, DyadicOperator }

#[derive(Debug)]
pub(crate) struct Definition { pub bodies: Vec<Vec<Statement>>, pub span: Span, pub kind: DefinitionKind, locals: &'static [Name], remembered: AtomicU64 }
impl Definition {
    /// The names that the definition could assign, sorted, which each of its calls keeps in slots.
    pub fn locals(&self) -> &'static [Name] { self.locals }
    /// What maps remember of a stopped batched run of this definition, as `batch::tried` encodes it: nothing, every later call, or the arguments whose later tries it skips (`transform::Cause::kept`).
    pub fn remembered(&self) -> u64 { self.remembered.load(Relaxed) }
    /// Records what maps remember of a stopped batched run of this definition.
    pub fn remember(&self, memory: u64) { self.remembered.store(memory, Relaxed) }
}

fn definition_kind(nodes: &[Node]) -> DefinitionKind {
    nodes.iter().map(|n| match &n.kind {
            NodeKind::Name(Name::RIGHT_OPERAND) => DefinitionKind::DyadicOperator,
            NodeKind::Name(Name::LEFT_OPERAND) => DefinitionKind::MonadicOperator,
            NodeKind::Group(nodes) | NodeKind::Run(nodes) => definition_kind(nodes),
            NodeKind::ArrayLiteral { cells, .. } => cells.iter().map(|nodes| definition_kind(nodes)).max().unwrap_or(DefinitionKind::Function),
            _ => DefinitionKind::Function, // Nested definitions classify their own bodies.
        })
        .max()
        .unwrap_or(DefinitionKind::Function)
}

/// The names that a definition's statements could assign, sorted: each name before an `←`, also inside a group, run or brackets there, but not inside a nested definition, which has frames of its own. A call gives each a slot. Some name a modified assignment's function, or the array that a selective assignment selects from, and their slots stay empty.
fn locals(bodies: &[Vec<Statement>]) -> &'static [Name] {
    let mut names = Vec::new();
    for statement in bodies.iter().flatten() { assigned(&statement.nodes, false, &mut names); }
    names.retain(|name| name.implicit().is_none());
    names.sort_unstable();
    names.dedup();
    crate::name::kept(&names)
}

/// Adds to `names` each name in `nodes` that comes before an `←`, or every name where `target` says that `nodes` themselves do.
fn assigned(nodes: &[Node], target: bool, names: &mut Vec<Name>) {
    let end = nodes.iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).unwrap_or(0);
    for (i, node) in nodes.iter().enumerate() {
        let target = target || i < end;
        match &node.kind {
            NodeKind::Name(name) if target => names.push(*name),
            NodeKind::Group(nodes) | NodeKind::Run(nodes) => assigned(nodes, target, names),
            NodeKind::ArrayLiteral { cells, .. } => for nodes in cells { assigned(nodes, target, names); },
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Node { pub kind: NodeKind, pub span: Span }

/// Structural syntax, not an arithmetic AST. Name roles will be resolved during binding.
#[derive(Clone, Debug)]
pub struct Parsed { pub(crate) statements: Vec<Statement> }

#[derive(Clone, Debug)]
/// A statement. A dfn's statement compiles its binder steps into `recording` on its first evaluation (`eval::Recording`), and later evaluations run them.
pub(crate) struct Statement { pub nodes: Vec<Node>, pub kind: StatementKind, pub(crate) recording: std::sync::OnceLock<Option<crate::eval::Recording>> }

#[derive(Clone, Copy, Debug)]
/// A predicate ends with `?`. When it fails, the dfn moves on to its next body.
pub(crate) enum StatementKind { Expression, DefaultArgument, Predicate, ErrorGuard { index: usize } }

fn statement(nodes: Vec<Node>) -> Result<Statement, ParseFailure> {
    let mut guard = None;
    for (i, node) in nodes.iter().enumerate() {
        if matches!(node.kind, NodeKind::ErrorGuard) {
            if i == 0 { return Err(invalid(&node.span, "error guard needs error numbers")); }
            if guard.is_some() { return Err(invalid(&node.span, "a statement can contain only one error guard")); }
            guard = Some(i);
        }
    }
    let kind = if let Some(index) = guard {
        StatementKind::ErrorGuard { index }
    } else if matches!(nodes[0].kind, NodeKind::Name(Name::ALPHA)) && matches!(nodes.get(1).map(|n| &n.kind), Some(NodeKind::Assign)) {
        StatementKind::DefaultArgument
    } else { StatementKind::Expression };
    Ok(Statement { nodes, kind, recording: Default::default() })
}

#[derive(Debug)]
pub enum ParseStatus { Complete(Parsed), Incomplete(Error), Invalid(Error) }

impl ParseStatus {
    /// The program, or the error for input that is incomplete or invalid.
    pub(crate) fn complete(self) -> Result<Parsed, Error> {
        match self { Self::Complete(parsed) => Ok(parsed), Self::Incomplete(e) | Self::Invalid(e) => Err(e) }
    }
}

#[derive(Debug)]
enum TokenKind {
    BraceOpen,
    BraceClose,
    Literal(Value),
    Function(Primitive),
    Operator(OperatorKind),
    /// A subscript integer, such as `₁` or `₋₁`, which selects a major cell of the item just before it.
    Subscript(i64),
    /// `ⱼ`, `ₑ` or `ᵣ` after an item other than a literal number.
    Joiner(Joiner),
    Open,
    /// A closing parenthesis, and the number marker after it.
    Close(Option<Mark>),
    BracketOpen,
    /// A closing bracket, and the number marker after it.
    BracketClose(Option<Mark>),
    /// `⋄`, or a line break between two items.
    Separator,
    Semicolon,
    Name(String),
    System(String),
    Assign,
    Output,
    ErrorGuard,
    Predicate,
}

struct Token { kind: TokenKind, span: Span }

#[derive(Clone)]
struct Characters<'a> { chars: Peekable<CharIndices<'a>>, aliases: bool }
impl<'a> Characters<'a> {
    fn new(text: &'a str, aliases: bool) -> Self { Self { chars: text.char_indices().peekable(), aliases } }
    fn spelling(&self) -> fn(char) -> char { if self.aliases { canonical } else { std::convert::identity } }
}
impl<'a> std::ops::Deref for Characters<'a> { type Target = Peekable<CharIndices<'a>>; fn deref(&self) -> &Self::Target { &self.chars } }
impl std::ops::DerefMut for Characters<'_> { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.chars } }
impl Iterator for Characters<'_> { type Item = (usize, char); fn next(&mut self) -> Option<Self::Item> { self.chars.next() } }

fn digits(chars: &mut Characters<'_>) -> usize {
    let mut count = 0;
    while chars.peek().is_some_and(|(_, c)| c.is_ascii_digit()) {
        chars.next();
        count += 1;
    }
    count
}

fn real_literal(chars: &mut Characters<'_>) -> Result<(), &'static str> {
    let canonical = chars.spelling();
    if chars.peek().is_some_and(|(_, c)| canonical(*c) == '¯') { chars.next(); }
    if chars.peek().is_some_and(|(_, c)| canonical(*c) == '∞') { chars.next(); return Ok(()); }
    let mut count = digits(chars);
    if chars.peek().is_some_and(|(_, c)| canonical(*c) == '.') && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit()) {
        chars.next();
        count += digits(chars);
    }
    if count == 0 { return Err("expected digits in numeric literal"); }
    if suffix(chars, EXPONENT) {
        chars.next_if(|&(_, c)| canonical(c) == '¯');
        if digits(chars) == 0 { return Err("expected exponent digits (use ¯ for a negative exponent)"); }
    }
    Ok(())
}

/// What a number marker after a literal makes its numbers: exact, or floats of one width.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mark { Exact, Width(FloatWidth) }
impl Mark {
    /// The marker's subscript.
    fn marker(self) -> char { match self { Self::Exact => EXACT.1, Self::Width(w) => width_marker(w) } }
    /// The zero of the marked numbers, the prototype of an empty marked list.
    fn zero(self) -> Number { match self { Self::Exact => Number::from_integer(0), Self::Width(w) => Number::float(0.0, w) } }
}

/// `ⱼ`, `ₑ` or `ᵣ` between two items, as in `xⱼy`. The binder joins the items into one term (`joined_term`).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Joiner { Imaginary, Exponent, Denominator }

/// The group that `left`, `joiner` at `at` and `right` stand for: `xⱼy` is `x+y×0ⱼ1`, `xₑy` is `x×10*y` and `xᵣy` is `xₓ÷yₓ`.
pub(crate) fn joined_term(left: Node, joiner: Joiner, at: &Span, right: Node) -> Node {
    let span = Span { source: at.source.clone(), range: left.span.range.start..right.span.range.end };
    let node = |kind| Node { kind, span: at.clone() };
    let number = |text| node(NodeKind::Literal(Value::Number(literal_number(text, Numeric::current()).expect("a joiner's constant is a number"))));
    let function = |p| node(NodeKind::Function(Builtin::Primitive(p)));
    let floor = || node(NodeKind::Operator(OperatorKind::Super(Superscript::Floor)));
    let nodes = match joiner {
        Joiner::Imaginary => vec![left, function(Primitive::Arithmetic(Arithmetic::Plus)), right, function(Primitive::Arithmetic(Arithmetic::Times)), number("0j1")],
        Joiner::Exponent => vec![left, function(Primitive::Arithmetic(Arithmetic::Times)), number("10"), function(Primitive::Math(Math::Power)), right],
        Joiner::Denominator => vec![left, floor(), function(Primitive::Arithmetic(Arithmetic::Divide)), right, floor()],
    };
    Node { kind: NodeKind::Group(nodes), span }
}

/// Consumes a number marker when one comes next.
fn mark(chars: &mut Characters<'_>) -> Option<Mark> { if suffix(chars, EXACT) { return Some(Mark::Exact); } width_mark(chars).map(Mark::Width) }

/// Consumes a float width's marker when one comes next, and gives the width.
fn width_mark(chars: &mut Characters<'_>) -> Option<FloatWidth> { WIDTHS.iter().find(|&&(s, _)| suffix(chars, s)).map(|&(_, w)| w) }

/// Whether `chars` starts a part of a number: a digit, `¯`, `∞`, or a point before a digit.
fn number_part(mut chars: Characters<'_>) -> bool {
    let canonical = chars.spelling();
    match chars.next().map(|(_, c)| canonical(c)) {
        Some(c) if c.is_ascii_digit() || matches!(c, '¯' | '∞') => true, Some('.') => chars.next().is_some_and(|(_, c)| c.is_ascii_digit()), _ => false
    }
}

/// Whether `suffix` comes next. A suffix with no plain letters, such as `ₓ`, ends a number and comes next wherever it appears. A suffix with plain letters joins two numbers, as in `1e5` or `1ⱼ2`, and comes next only before a number part: `2edges` is `2` beside `edges`, and `2ₑn` is `2` before `ₑ`, which joins `2` and `n`.
fn suffix_follows(chars: &Characters<'_>, (plain, sub): Suffix) -> bool {
    let mut ahead = chars.clone();
    match ahead.next() { Some((_, c)) if c == sub || plain.contains(&c) => plain.is_empty() || number_part(ahead), _ => false }
}

/// Consumes `suffix` when it comes next, and gives whether it did.
fn suffix(chars: &mut Characters<'_>, suffix: Suffix) -> bool {
    let follows = suffix_follows(chars, suffix);
    if follows { chars.next(); }
    follows
}

/// The number that literal text writes, read by `Number::literal` once each character takes its canonical spelling.
fn literal_number(text: &str, numeric: Numeric) -> Result<Number, ErrorKind> {
    Number::literal(&text.chars().map(canonical).collect::<String>(), numeric)
}

/// A number written inside `[…]ₓ`. An integer's digits read exactly, and a whole float becomes exact.
fn exact_number(text: &str, numeric: Numeric) -> Result<Number, ErrorKind> {
    let numeric = numeric.with_width(FloatWidth::F64);
    if text.chars().all(|c| c.is_ascii_digit() || canonical(c) == '¯') {
        return Number::parse(&format!("{}ₓ", text.chars().map(canonical).collect::<String>()), numeric);
    }
    literal_number(text, numeric)?.marked_exact().ok_or_else(|| Kind::Domain.into())
}

/// The empty numeric vector whose prototype is `fill`.
pub(crate) fn zilde(fill: Number) -> Value { Value::empty([0], Value::Number(fill)).unwrap() }

/// Whether a token can be part of a marked literal: a literal, a parenthesis or bracket, `;`, `⋄`, a key's colon, or a superscript that the parser folds into the number before it.
fn literal_token(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Literal(_) | TokenKind::Open | TokenKind::Close(_) | TokenKind::BracketOpen | TokenKind::BracketClose(_)
        | TokenKind::Semicolon | TokenKind::Separator | TokenKind::Function(Primitive::Keys)
        | TokenKind::Operator(OperatorKind::Super(Superscript::Power(_) | Superscript::Transpose)))
}

/// Reads the numbers and `⍬`s inside each `[…]` and `(…)` that has a number marker again, from their own source text, at any depth: as exact after `ₓ`, and as floats or complex numbers of the marked width after `ₕ`, `ₛ` or `ₚ`. Booleans stay as they are. The `ₓ` after a group that holds anything but literals (`literal_token`) is `⌊`, as after a name. Gives the byte ranges of those `ₓ`s, for highlighting.
fn mark_literals(tokens: &mut Vec<Token>) -> Result<Vec<std::ops::Range<usize>>, Error> {
    let numeric = Numeric::current();
    let (mut open, mut floors, mut floored) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..tokens.len() {
        match tokens[i].kind {
            TokenKind::Open | TokenKind::BracketOpen => open.push(i),
            TokenKind::Close(mark) | TokenKind::BracketClose(mark) => {
                let Some(start) = open.pop() else { continue };
                let Some(mark) = mark else { continue };
                if mark == Mark::Exact && !tokens[start + 1..i].iter().all(|t| literal_token(&t.kind)) { floors.push(i); continue }
                for token in &mut tokens[start + 1..i] {
                    let text = token.span.text();
                    let value = match (&token.kind, mark) {
                        (TokenKind::Literal(Value::Number(n)), _) if n.as_bool().is_some() => continue,
                        (TokenKind::Literal(Value::Number(_)), Mark::Exact) if text.starts_with('$') => continue,
                        (TokenKind::Literal(Value::Number(_)), Mark::Exact) => {
                            Value::Number(exact_number(text, numeric).error_at(&token.span, "ₓ marks whole numbers exact")?)
                        }
                        (TokenKind::Literal(Value::Number(n)), Mark::Width(w)) => {
                            let n = if text.starts_with('$') { Ok(n.clone()) } else { literal_number(text, numeric.with_width(w)) };
                            let n = n.and_then(|n| n.to_float_width(w).map_err(|_| Kind::Domain.into()));
                            Value::Number(n.error_at(&token.span, &format!("{} marks numbers", mark.marker()))?)
                        }
                        (TokenKind::Literal(_), _) if text.chars().map(canonical).eq(['⍬']) => zilde(mark.zero()),
                        _ => continue,
                    };
                    token.kind = TokenKind::Literal(value);
                }
            }
            _ => (),
        }
    }
    // Each such group loses its mark. Its `ₓ` becomes a token of its own, the `⌊` that `ₓ` is after a name.
    for i in floors.into_iter().rev() {
        let close = &mut tokens[i];
        close.kind = match close.kind { TokenKind::Close(_) => TokenKind::Close(None), _ => TokenKind::BracketClose(None) };
        let end = close.span.range.end;
        close.span.range.end = end - EXACT.1.len_utf8();
        let marker = Span { source: close.span.source.clone(), range: close.span.range.end..end };
        floored.push(marker.range.clone());
        tokens.insert(i + 1, Token { kind: TokenKind::Operator(OperatorKind::Super(Superscript::Floor)), span: marker });
    }
    Ok(floored)
}

/// The offset of the next character, or the end of the text.
fn position(chars: &mut Characters<'_>, len: usize) -> usize { chars.peek().map_or(len, |&(i, _)| i) }

/// The end of a character that would run on from a numeric literal: a point, a numeric suffix, or a digit when `digit` is set.
fn run_on(chars: &mut Characters<'_>, digit: bool) -> Option<usize> {
    let canonical = chars.spelling();
    let &(i, c) = chars.peek()?;
    let suffix = SUFFIXES.into_iter().any(|s| suffix_follows(chars, s));
    ((digit && c.is_ascii_digit()) || canonical(c) == '.' || suffix).then(|| i + c.len_utf8())
}

/// Whether `c` can start or continue a name. Syntax, glyphs and digits end a name.
pub(crate) fn name_char(c: char) -> bool {
    !c.is_whitespace() && !c.is_control() && !c.is_ascii_digit() && !subscript(c)
        && !matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '\'' | '"' | ';' | ':' | '?' | '$' | '•' | '←' | '⎕' | '⍝' | '⋄'
            | '⍬' | '¯' | '∞' | '⍛' | 'ᵀ' | 'ᵘ' | '⁻' | '₋')
        && !is_implicit(c)
        && script_digit(SUPERSCRIPT_DIGITS, c).is_none() && script_digit(SUBSCRIPT_DIGITS, c).is_none()
        && Primitive::from_glyph(c).is_none() && OperatorKind::from_glyph(c).is_none()
}

pub(crate) fn name_char_in(c: char, aliases: bool) -> bool { name_char(if aliases { canonical(c) } else { c }) }

const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
const SUBSCRIPT_DIGITS: &str = "₀₁₂₃₄₅₆₇₈₉";
/// The characters that write a superscript operator, `ₓ` among them, for the editors' glyph lists.
pub fn superscripts() -> String { [SUPERSCRIPT_DIGITS, "⁻ᵀᵘₓ"].concat() }
/// The characters that write a subscript, for the editors' glyph lists.
pub fn subscripts() -> String { [SUBSCRIPT_DIGITS, "₋"].concat() }

/// The value of `c` among `digits`, such as `²` among the superscript digits.
fn script_digit(digits: &str, c: char) -> Option<i64> { digits.chars().position(|d| d == c).map(|i| i as i64) }

/// The integer that a superscript or subscript writes, from its first character `c`: a digit, or the minus (`⁻` or `₋`) of a negative integer. `what` names the kind in errors.
fn script_integer(c: char, chars: &mut Characters<'_>, digits: &str, what: &str) -> Result<i64, (ErrorKind, String)> {
    let (mut value, mut count) = script_digit(digits, c).map_or((0, 0), |d| (d, 1));
    while let Some(d) = chars.peek().and_then(|&(_, c)| script_digit(digits, c)) {
        chars.next();
        count += 1;
        value = value.checked_mul(10).and_then(|v| v.checked_add(d)).ok_or_else(|| (ErrorKind::from(Kind::Limit), format!("{what} is too large")))?;
    }
    if count == 0 { return Err((ErrorKind::Syntax, format!("{c} needs a {what} digit after it"))); }
    Ok(if script_digit(digits, c).is_some() { value } else { -value })
}

const UNQUOTED: &str = "_ quotes the next character into a name, and there is none";
const DOLLAR: &str = "$ and one letter is a constant ($t, $f, $n, $a or $d) or a handler's error ($e)";

/// Reads the rest of a name. Unreserved characters continue it, and a digit ends it. `_` quotes the next character into the name, and a quoted digit takes the digits after it, so `x_12` is one name. After a dot, digits continue a key, so `T.q1` names the key `q1`. A lone `_`, before a space, `]`, `)`, `←`, `;`, `⋄` or the end, is the name `_`. Any other `_` with nothing to quote gives the position after it.
fn name(chars: &mut Characters<'_>, key: bool) -> Result<(), (usize, &'static str)> {
    let canonical = chars.spelling();
    let aliases = chars.aliases;
    let lone = |c: Option<&(usize, char)>| c.is_none_or(|&(_, c)| c.is_whitespace() || matches!(canonical(c), ']' | ')' | '←' | ';' | '⋄'));
    let mut first = true;
    loop {
        match chars.peek() {
            Some(&(_, '_')) => {
                let (i, _) = chars.next().unwrap();
                if first && lone(chars.peek()) { return Ok(()); }
                match chars.next() {
                    Some((_, q)) if !q.is_whitespace() => { if q.is_ascii_digit() { digits(chars); } } _ => return Err((i + 1, UNQUOTED))
                }
            }
            Some(&(_, c)) if name_char_in(c, aliases) || (key && c.is_ascii_digit()) => { chars.next(); }
            _ => return Ok(()),
        }
        first = false;
    }
}

/// Whether a line break beside a token of `kind` separates nothing. `before` says the token comes before the line break. Every other line break reads as `⋄`.
fn absorbs_line_break(kind: &TokenKind, before: bool) -> bool {
    use TokenKind::*;
    match kind {
        Separator | Semicolon | Predicate => true, Open | BracketOpen | BraceOpen => before, Close(_) | BracketClose(_) | BraceClose => !before, _ => false
    }
}

/// The literal record of a leading `•prefs` statement.
pub(crate) fn prefs_record(nodes: &[Node]) -> Option<&Node> {
    let nodes = match nodes { [Node { kind: NodeKind::Run(inner), .. }] => inner.as_slice(), _ => nodes };
    match nodes {
        [Node { kind: NodeKind::Function(Builtin::System(crate::system::Entry::PREFS)), .. }, record @ Node { kind: NodeKind::ArrayLiteral { record: true, .. }, .. }]
            if non_literal(std::slice::from_ref(record), &|_| false).is_none() => Some(record), _ => None
    }
}

/// Whether `tokens` hold a `•prefs` statement on a literal record. If so, its `hanzi_aliases` setting goes into `aliases`, and its numeric settings come into force, for the tokens after it to read under. A setting with an invalid value is left as it was, for the statement's evaluation to report.
fn header(tokens: &[Token], aliases: &mut bool) -> bool {
    let Some(start) = tokens.iter().position(|t| !matches!(t.kind, TokenKind::Separator)) else { return false };
    let tokens = &tokens[start..];
    if !matches!(tokens.first().map(|t| &t.kind), Some(TokenKind::System(name)) if crate::system::Entry::find(name) == Some(crate::system::Entry::PREFS)) { return false; }
    let Ok(statements) = (Parser { tokens, pos: 0 }).pieces(None).and_then(statements) else { return false };
    let [nodes] = statements.as_slice() else { return false };
    let Some(record) = prefs_record(nodes) else { return false };
    let NodeKind::ArrayLiteral { cells, .. } = &record.kind else { unreachable!() };
    let mut numeric = Numeric::current();
    for nodes in cells {
        let [Node { kind: NodeKind::Literal(key), .. }, Node { kind: NodeKind::Function(Builtin::Primitive(Primitive::Keys)), .. }, Node { kind: NodeKind::Literal(value), .. }] = nodes.as_slice() else { continue };
        let Some(key) = crate::keyed::name(key) else { continue };
        if &*key == "hanzi_aliases" { if let Ok(on) = value.boolean() { *aliases = on; } } else { let _ = numeric.set(&key, value); }
    }
    crate::settings::set_numeric(numeric);
    true
}

type Highlights = Vec<(std::ops::Range<usize>, &'static str)>;

/// The tokens of `source`. Each leading `•prefs` statement on a literal record sets the hanzi aliases and the numeric settings that the tokens after it read under, as `header` applies them.
fn lex(source: &Arc<Source>, aliases: bool, highlights: Option<&mut Highlights>) -> Result<Vec<Token>, ParseFailure> {
    lexed(source, aliases, highlights)
}

fn lexed(source: &Arc<Source>, aliases: bool, mut highlights: Option<&mut Highlights>) -> Result<Vec<Token>, ParseFailure> {
    let len = source.text.len();
    let mut chars = Characters::new(&source.text, aliases);
    // A script's first line can name its interpreter.
    if source.text.starts_with("#!") { while chars.next_if(|&(_, c)| c != '\n').is_some() {} }
    let (mut tokens, mut line_break) = (Vec::new(), None);
    let (mut leading, mut prefix_start, mut depth) = (true, 0, 0usize);
    while let Some(&(start, c)) = chars.peek() {
        let span = |end| Span { source: source.clone(), range: start..end };
        if c == '\n' {
            if leading && depth == 0 && tokens.len() > prefix_start {
                leading = header(&tokens[prefix_start..], &mut chars.aliases);
                prefix_start = tokens.len();
            }
            chars.next();
            if tokens.last().is_some_and(|t: &Token| !absorbs_line_break(&t.kind, true)) { line_break = Some(span(start + 1)); }
            continue;
        }
        let canonical = chars.spelling();
        let kind = match token(source, &mut chars) {
            Ok(kind) => kind,
            Err(failure) => {
                let Some(classes) = highlights.as_deref_mut() else { return Err(failure); };
                let (ParseFailure::Incomplete(e) | ParseFailure::Invalid(e)) = failure;
                let end = e.span.range.end.max(start + c.len_utf8());
                while chars.next_if(|&(i, _)| i < end).is_some() {}
                if matches!(c, '"' | '\'') { classes.push((start..position(&mut chars, len), "string")); }
                continue;
            }
        };
        if let Some(classes) = highlights.as_deref_mut() {
            let class = match &kind {
                Some(kind) => class(kind, &source.text[start..position(&mut chars, len)], canonical), None => (canonical(c) == '⍝').then_some("comment")
            };
            if let Some(class) = class { classes.push((start..position(&mut chars, len), class)); }
            // A group's mark is part of its literal, unless `mark_literals` finds that the group holds anything else.
            if let Some(TokenKind::Close(Some(_)) | TokenKind::BracketClose(Some(_))) = &kind { classes.push((start + c.len_utf8()..position(&mut chars, len), "literal")); }
        }
        let Some(kind) = kind else { continue };
        if let Some(at) = line_break.take().filter(|_| !absorbs_line_break(&kind, false)) { tokens.push(Token { kind: TokenKind::Separator, span: at }); }
        match kind {
            TokenKind::Open | TokenKind::BracketOpen | TokenKind::BraceOpen => depth += 1,
            TokenKind::Close(_) | TokenKind::BracketClose(_) | TokenKind::BraceClose => depth = depth.saturating_sub(1), _ => (),
        }
        if leading && depth == 0 && matches!(kind, TokenKind::Separator) {
            leading = header(&tokens[prefix_start..], &mut chars.aliases);
            prefix_start = tokens.len() + 1;
        }
        tokens.push(Token { kind, span: span(position(&mut chars, len)) });
    }
    let floored = mark_literals(&mut tokens)?;
    if let Some(classes) = highlights { for (range, class) in classes.iter_mut() { if floored.contains(range) { *class = "monadic-operator"; } } }
    Ok(tokens)
}

/// Reads the token that starts `chars`, or `None` after white space or a comment.
fn token(source: &Arc<Source>, chars: &mut Characters<'_>) -> Result<Option<TokenKind>, ParseFailure> {
    let canonical = chars.spelling();
    let numeric = Numeric::current();
    let len = source.text.len();
    let Some(&(start, raw)) = chars.peek() else { return Ok(None) };
    let c = canonical(raw);
    let span = |end| Span { source: source.clone(), range: start..end };
    Ok(Some(if c.is_ascii_digit() || matches!(c, '¯' | '∞') || (c == '.' && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit())) {
        real_literal(chars).map_err(|message| span(position(chars, len)).error(ErrorKind::Syntax, message))?;
        if suffix(chars, IMAGINARY) {
            real_literal(chars).map_err(|message| span(position(chars, len)).error(ErrorKind::Syntax, message))?;
            width_mark(chars);
            if let Some(end) = run_on(chars, false) { return Err(span(end).error(ErrorKind::Syntax, "invalid complex numeric literal").into()); }
        } else {
            let rational = suffix(chars, DENOMINATOR);
            if rational {
                chars.next_if(|&(_, c)| canonical(c) == '¯');
                if digits(chars) == 0 { return Err(span(position(chars, len)).error(ErrorKind::Syntax, "expected integer denominator").into()); }
            }
            if rational || mark(chars).is_some() {
                if let Some(end) = run_on(chars, true) { return Err(span(end).error(ErrorKind::Syntax, "invalid exact numeric literal").into()); }
            }
        }
        let end = position(chars, len);
        let n = literal_number(&source.text[start..end], numeric)
            .error_at(&span(end), "invalid numeric literal (real values, finite complex components or integer components with ₓ and r required)")?;
        TokenKind::Literal(Value::number(n).unwrap())
    } else if name_char_in(raw, chars.aliases) {
        let key = source.text[..start].chars().next_back().is_some_and(|c| canonical(c) == '.');
        name(chars, key).map_err(|(end, message)| span(end).error(ErrorKind::Syntax, message))?;
        TokenKind::Name(source.text[start..position(chars, len)].to_owned())
    } else {
        chars.next();
        match c {
            '\'' => {
                let unclosed = |at| Err(span(at).error(ErrorKind::Syntax, "unclosed character literal").into());
                let c = match chars.next() { Some((_, c)) if c != '\n' => c, next => return unclosed(next.map_or(len, |(i, _)| i)) };
                if c == '\'' && chars.peek().is_none_or(|&(_, c)| c != '\'') { TokenKind::Literal(crate::keyed::text("")) } else {
                    match chars.peek() {
                        Some((_, '\'')) => { chars.next(); }
                        None | Some((_, '\n')) => return unclosed(position(chars, len)),
                        Some(&(i, c)) => {
                            return Err(span(i + c.len_utf8()).error(ErrorKind::Syntax, "a character literal holds one character (strings use double quotes)")
                                .into())
                        }
                    }
                    TokenKind::Literal(Value::Character(c))
                }
            }
            '"' => {
                let mut text = String::new();
                loop {
                    match chars.next() {
                        Some((_, '"')) if chars.next_if(|&(_, c)| c == '"').is_some() => text.push('"'),
                        Some((_, '"')) => break,
                        // Each line ending, whatever its form, reads as one newline. Indentation stays as written.
                        Some((_, '\r')) => {
                            chars.next_if(|&(_, c)| c == '\n');
                            text.push('\n');
                        }
                        Some((_, c)) => text.push(c),
                        None => return Err(ParseFailure::Incomplete(span(len).error(ErrorKind::Syntax, "unclosed string"))),
                    }
                }
                TokenKind::Literal(crate::keyed::text(&text))
            }
            '⍬' => TokenKind::Literal(zilde(mark(chars).map_or(numeric.float(0.0), Mark::zero))),
            // `$` and one letter: a literal constant, or `$e`, the error a handler caught. `$n` and `$m` take a width mark.
            '$' => {
                let token = match chars.next() {
                    Some((_, 't')) => TokenKind::Literal(Value::Number(Number::from_bool(true))),
                    Some((_, 'f')) => TokenKind::Literal(Value::Number(Number::from_bool(false))),
                    Some((_, 'n')) => TokenKind::Literal(Value::Number(Number::float(f64::NAN, width_mark(chars).unwrap_or(numeric.width)))),
                    Some((_, 'a')) => TokenKind::Literal(crate::keyed::text("ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
                    Some((_, 'd')) => TokenKind::Literal(crate::keyed::text("0123456789")),
                    Some((_, 'm')) => {
                        let width = width_mark(chars).unwrap_or(numeric.width);
                        TokenKind::Literal(Value::Number(Number::float(width.epsilon(), width)))
                    }
                    Some((_, 'e')) => TokenKind::Name("$e".into()),
                    _ => return Err(span(position(chars, len)).error(ErrorKind::Syntax, DOLLAR).into()),
                };
                if chars.peek().is_some_and(|&(_, c)| name_char(c) || c.is_ascii_digit()) {
                    return Err(span(position(chars, len)).error(ErrorKind::Syntax, DOLLAR).into());
                }
                token
            }
            '⍛' => return Err(span(start + c.len_utf8()).error(ErrorKind::Syntax, "⍛ is retired: use ↣ or ↢ to bind or preprocess, and ∘ for Atop").into()),
            '(' => TokenKind::Open,
            ')' => TokenKind::Close(mark(chars)),
            '[' => TokenKind::BracketOpen,
            ']' => TokenKind::BracketClose(mark(chars)),
            ';' => TokenKind::Semicolon,
            '⋄' => TokenKind::Separator,
            '←' => TokenKind::Assign,
            '•' if chars.peek().is_some_and(|&(_, c)| name_char(c)) => {
                name(chars, false).map_err(|(end, message)| span(end).error(ErrorKind::Syntax, message))?;
                TokenKind::System(format!("•{}", &source.text[start + raw.len_utf8()..position(chars, len)]))
            }
            '⎕' => TokenKind::Output,
            ':' if chars.peek().is_some_and(|(_, c)| canonical(*c) == ':') => { chars.next(); TokenKind::ErrorGuard }
            ':' => TokenKind::Function(Primitive::Keys),
            '?' => TokenKind::Predicate,
            '{' => TokenKind::BraceOpen,
            '}' => TokenKind::BraceClose,
            c if is_implicit(c) => TokenKind::Name(c.to_string()),
            '⍝' => {
                while chars.peek().is_some_and(|(_, c)| *c != '\n') { chars.next(); }
                return Ok(None);
            }
            'ᵀ' => TokenKind::Operator(OperatorKind::Super(Superscript::Transpose)),
            'ᵘ' => TokenKind::Operator(OperatorKind::Super(Superscript::Unit)),
            // A literal number, `)` and `]` take the `ₓ` after them (`mark_literals`). An `ₓ` that reaches here follows another item, such as a name, and is `⌊`.
            'ₓ' => TokenKind::Operator(OperatorKind::Super(Superscript::Floor)),
            // A literal number takes `ⱼ`, `ₑ` or `ᵣ` only before a number part (`suffix_follows`). One that reaches here joins two items, as in `xⱼy`.
            'ⱼ' => TokenKind::Joiner(Joiner::Imaginary),
            'ₑ' => TokenKind::Joiner(Joiner::Exponent),
            'ᵣ' => TokenKind::Joiner(Joiner::Denominator),
            c if c == '⁻' || script_digit(SUPERSCRIPT_DIGITS, c).is_some() => {
                let power = script_integer(c, chars, SUPERSCRIPT_DIGITS, "superscript").map_err(|(k, m)| span(position(chars, len)).error(k, m))?;
                TokenKind::Operator(OperatorKind::Super(Superscript::Power(power)))
            }
            c if c == '₋' || script_digit(SUBSCRIPT_DIGITS, c).is_some() => {
                TokenKind::Subscript(script_integer(c, chars, SUBSCRIPT_DIGITS, "subscript").map_err(|(k, m)| span(position(chars, len)).error(k, m))?)
            }
            c if c.is_whitespace() => return Ok(None),
            c => match (OperatorKind::from_glyph(c), Primitive::from_glyph(c)) {
                (Some(op), _) => TokenKind::Operator(op), (_, Some(f)) => TokenKind::Function(f),
                _ => return Err(span(start + raw.len_utf8()).error(ErrorKind::Unsupported, format!("{raw:?} is not supported yet")).into())
            },
        }
    }))
}

/// Each token and comment of `text` with its highlight class, as byte ranges in order. The class is the kind that `symbols` gives a glyph, or `"literal"` for a number or a joiner, `"string"` for a character or string literal, `"name"` for a name, `"function"` for a system name, as for a glyph function, `"monadic-operator"` for a superscript operator, `"syntax"` for a subscript or an error guard, and `"comment"` for a comment. Brackets have no class. Text that fails to lex has none either, except an unclosed literal, which is a `"string"` up to where lexing stopped.
pub fn highlight(text: &str, aliases: bool) -> Vec<(std::ops::Range<usize>, &'static str)> {
    let mut classes = Vec::new();
    let _ = lex(&Source::new("<highlight>", text), aliases, Some(&mut classes));
    classes
}

/// A token's highlight class: its glyph's kind in `symbols`, or the class its kind of token implies.
fn class(kind: &TokenKind, text: &str, canonical: fn(char) -> char) -> Option<&'static str> {
    if let Some(symbol) = text.parse::<char>().ok().and_then(|c| crate::symbols::symbols().iter().find(|s| s.glyph.chars().eq([canonical(c)]))) {
        return Some(symbol.kind);
    }
    match kind {
        TokenKind::Literal(_) => Some(if text.starts_with(['"', '\'']) { "string" } else { "literal" }),
        TokenKind::Operator(OperatorKind::Super(_)) => Some("monadic-operator"), TokenKind::Subscript(_) | TokenKind::ErrorGuard => Some("syntax"),
        TokenKind::Joiner(_) => Some("literal"),
        TokenKind::Name(_) => Some("name"), TokenKind::System(_) => Some("function"), _ => None
    }
}

/// Whether the end of `text` is in code, not inside a comment or an unclosed string or character literal.
pub fn in_code(text: &str, aliases: bool) -> bool {
    match highlight(text, aliases).pop() {
        Some((range, "comment")) => range.end < text.len(),
        Some((range, "string")) if range.end == text.len() => lex(&Source::new("<literal>", &text[range]), aliases, None).is_ok(), _ => true
    }
}

enum ParseFailure { Incomplete(Error), Invalid(Error) }

fn invalid(span: &Span, message: &str) -> ParseFailure { ParseFailure::Invalid(span.error(ErrorKind::Syntax, message)) }
impl From<Error> for ParseFailure { fn from(e: Error) -> Self { Self::Invalid(e) } }
impl From<ParseFailure> for ParseStatus {
    fn from(failure: ParseFailure) -> Self {
        match failure { ParseFailure::Incomplete(e) => Self::Incomplete(e), ParseFailure::Invalid(e) => Self::Invalid(e) }
    }
}

/// The span from the earliest node's start to the latest node's end, whatever order the nodes are in. The binder's rewrites can put a later node first, as `.[I]` puts its brackets before the array.
pub(crate) fn cover(nodes: &[Node]) -> Span {
    let start = nodes.iter().map(|n| n.span.range.start).min().expect("at least one node");
    let end = nodes.iter().map(|n| n.span.range.end).max().expect("at least one node");
    Span { source: nodes[0].span.source.clone(), range: start..end }
}

/// Nodes and separators, before the enclosing delimiters give the separators their meaning.
enum Piece { Node(Node), Diamond, Semicolon(Span), Predicate(Span) }

struct Parser<'a> { tokens: &'a [Token], pos: usize }
impl Parser<'_> {
    fn pieces(&mut self, open: Option<&Token>) -> Result<Vec<Piece>, ParseFailure> {
        let mut pieces = Vec::new();
        while let Some(token) = self.tokens.get(self.pos) {
            self.pos += 1;
            let mut span = token.span.clone();
            let kind = match &token.kind {
                TokenKind::Separator => { pieces.push(Piece::Diamond); continue; }
                TokenKind::Semicolon => { pieces.push(Piece::Semicolon(span)); continue; }
                TokenKind::Predicate => { pieces.push(Piece::Predicate(span)); continue; }
                TokenKind::Close(_) | TokenKind::BracketClose(_) | TokenKind::BraceClose => {
                    let matched = open.is_some_and(|o| {
                        matches!(
                            (&o.kind, &token.kind),
                            (TokenKind::Open, TokenKind::Close(_))
                                | (TokenKind::BracketOpen, TokenKind::BracketClose(_))
                                | (TokenKind::BraceOpen, TokenKind::BraceClose)
                        )
                    });
                    if !matched { return Err(invalid(&token.span, "mismatched closing delimiter")); }
                    return Ok(pieces);
                }
                TokenKind::Open | TokenKind::BracketOpen | TokenKind::BraceOpen => {
                    let inner = self.pieces(Some(token))?;
                    span.range.end = self.tokens[self.pos - 1].span.range.end;
                    let mark = match self.tokens[self.pos - 1].kind { TokenKind::Close(mark) | TokenKind::BracketClose(mark) => mark, _ => None };
                    match token.kind {
                        TokenKind::BraceOpen => {
                            let bodies = bodies(inner, &span)?;
                            let kind = bodies.iter().flatten().map(|s| definition_kind(&s.nodes)).max().unwrap_or(DefinitionKind::Function);
                            NodeKind::Dfn(Arc::new(Definition { locals: locals(&bodies), bodies, span: span.clone(), kind, remembered: AtomicU64::new(0) }))
                        }
                        TokenKind::Open if let Some(mark) = mark => marked_literal(parenthesised(inner, &span)?, &span, mark)?,
                        TokenKind::Open => parenthesised(inner, &span)?,
                        _ if let Some(mark) = mark => marked_literal(brackets(inner, &span)?, &span, mark)?,
                        _ => brackets(inner, &span)?,
                    }
                }
                TokenKind::Literal(a) => NodeKind::Literal(a.clone()),
                TokenKind::Function(f) => NodeKind::Function(Builtin::Primitive(*f)),
                TokenKind::Operator(op) => NodeKind::Operator(*op),
                TokenKind::Subscript(index) => NodeKind::Subscript(*index),
                TokenKind::Joiner(joiner) => NodeKind::Joiner(*joiner),
                TokenKind::Name(name) => NodeKind::Name(Name::new(name)),
                TokenKind::System(name) => crate::system::Entry::find(name).map_or_else(|| NodeKind::System(name.clone()), |entry| NodeKind::Function(Builtin::System(entry))),
                TokenKind::Assign => NodeKind::Assign,
                TokenKind::Output => NodeKind::Output,
                TokenKind::ErrorGuard => {
                    if !open.is_some_and(|o| matches!(o.kind, TokenKind::BraceOpen)) { return Err(invalid(&token.span, "error guards belong to dfns")); }
                    NodeKind::ErrorGuard
                }
            };
            pieces.push(Piece::Node(Node { kind, span }));
        }
        if let Some(open) = open { return Err(ParseFailure::Incomplete(open.span.error(ErrorKind::Syntax, "unclosed delimiter"))); }
        Ok(pieces)
    }
}

/// The nodes between separators. `⋄` always separates, and `;` only when `semicolons` is set. Elsewhere `;` is an error. `?` belongs only in dfns.
fn split(pieces: Vec<Piece>, semicolons: bool) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let mut parts = vec![Vec::new()];
    for piece in pieces {
        match piece {
            Piece::Node(n) => parts.last_mut().unwrap().push(n),
            Piece::Semicolon(s) if !semicolons => return Err(invalid(&s, "; separates items in brackets and bodies in braces, and nowhere else")),
            Piece::Predicate(s) => return Err(invalid(&s, "? ends a predicate, which belongs in a dfn")), _ => parts.push(Vec::new())
        }
    }
    Ok(parts)
}

/// Statements at the top level: a line break or `⋄` ends one.
fn statements(pieces: Vec<Piece>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    split(pieces, false)?.into_iter().filter(|nodes| !nodes.is_empty()).map(expression).collect()
}

/// A dfn's bodies, separated by `;`. A line break or `⋄` ends a statement, and `?` ends a predicate. Every body but the last needs a predicate: a body without one always returns, so the bodies after it could never run. A body that ends after its predicate returns no result.
fn bodies(pieces: Vec<Piece>, span: &Span) -> Result<Vec<Vec<Statement>>, ParseFailure> {
    let (mut bodies, mut ends, mut body, mut nodes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for piece in pieces {
        match piece {
            Piece::Node(n) => nodes.push(n),
            Piece::Diamond => end_statement(&mut body, &mut nodes, None)?,
            Piece::Predicate(s) => end_statement(&mut body, &mut nodes, Some(s))?,
            Piece::Semicolon(s) => {
                end_statement(&mut body, &mut nodes, None)?;
                bodies.push(std::mem::take(&mut body));
                ends.push(s);
            }
        }
    }
    end_statement(&mut body, &mut nodes, None)?;
    bodies.push(body);
    ends.push(span.clone());
    let predicate = |s: &Statement| matches!(s.kind, StatementKind::Predicate);
    for (i, (body, end)) in bodies.iter().zip(&ends).enumerate() {
        match body.last() {
            None if bodies.len() > 1 => return Err(invalid(end, "empty dfn body")),
            _ if i + 1 < bodies.len() && !body.iter().any(predicate) => {
                return Err(invalid(end, "a body without a predicate always returns, so the bodies after it never run"));
            }
            _ => (),
        }
    }
    Ok(bodies)
}

/// End the statement in `nodes`, which `predicate` marks as a condition.
fn end_statement(body: &mut Vec<Statement>, nodes: &mut Vec<Node>, predicate: Option<Span>) -> Result<(), ParseFailure> {
    if nodes.is_empty() { return predicate.map_or(Ok(()), |s| Err(invalid(&s, "a predicate needs a condition before ?"))); }
    let mut statement = statement(expression(std::mem::take(nodes))?)?;
    if let Some(s) = predicate {
        if !matches!(statement.kind, StatementKind::Expression) { return Err(invalid(&s, "a predicate must be an expression")); }
        statement.kind = StatementKind::Predicate;
    }
    body.push(statement);
    Ok(())
}

/// Parentheses group. With `⋄` or a line break between items, they write rows instead.
fn parenthesised(pieces: Vec<Piece>, span: &Span) -> Result<NodeKind, ParseFailure> {
    if pieces.iter().any(|p| matches!(p, Piece::Diamond)) { return rows(pieces, span); }
    let nodes = split(pieces, false)?.pop().unwrap();
    if nodes.is_empty() { return Err(invalid(span, "empty grouping is not a value")); }
    Ok(NodeKind::Group(expression(nodes)?))
}

/// Parts separated by `⋄`, where a trailing `⋄` ends the last row: `(1 2⋄)` is `[[1 2]]`, and `[1 2⋄]` is a 1×2 matrix.
fn diamond_rows(mut parts: Vec<Vec<Node>>, span: &Span) -> Result<Vec<Vec<Node>>, ParseFailure> {
    if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) { parts.pop(); }
    if parts.iter().any(Vec::is_empty) { return Err(invalid(span, "empty row between diamonds")); }
    Ok(parts)
}

/// One row read as a bracketed list of its items.
fn row(nodes: Vec<Node>) -> Result<Node, ParseFailure> {
    let span = cover(&nodes);
    let cells = items(nodes)?;
    Ok(Node { kind: NodeKind::ArrayLiteral { record: cells.iter().any(|c| key_colon(c).is_some()), cells, form: ListForm::Items }, span })
}

/// `(4⋄4 5)` is `[[4] [4 5]]`. Its items are the rows, each read as a bracketed list.
fn rows(pieces: Vec<Piece>, span: &Span) -> Result<NodeKind, ParseFailure> {
    let cells = diamond_rows(split(pieces, false)?, span)?.into_iter().map(|nodes| Ok(vec![row(nodes)?])).collect::<Result<_, ParseFailure>>()?;
    Ok(NodeKind::ArrayLiteral { cells, form: ListForm::Rows, record: false })
}

/// Whether nothing comes between two nodes in the source.
pub(crate) fn touching(a: &Node, b: &Node) -> bool { a.span.range.end == b.span.range.start }

/// Brackets build arrays. A space separates items, `;` separates items that contain spaces, and `⋄` or a line break separates major cells. Brackets round one item make a one-item vector.
fn brackets(pieces: Vec<Piece>, span: &Span) -> Result<NodeKind, ParseFailure> {
    let semicolon = pieces.iter().any(|p| matches!(p, Piece::Semicolon(_)));
    let diamond = pieces.iter().any(|p| matches!(p, Piece::Diamond));
    if semicolon && diamond { return Err(invalid(span, "; and ⋄ cannot both separate items in one pair of brackets")); }
    let mut parts = split(pieces, true)?;
    if diamond {
        // A row that is a single item is that item's major cell.
        let cells = diamond_rows(parts, span)?.into_iter().map(|nodes| {
            let row = row(nodes)?;
            Ok(match row.kind { NodeKind::ArrayLiteral { mut cells, .. } if cells.len() == 1 => cells.pop().unwrap(), _ => vec![row] })
        }).collect::<Result<_, ParseFailure>>()?;
        return Ok(NodeKind::ArrayLiteral { cells, form: ListForm::Cells, record: false });
    }
    let cells = if semicolon {
        if parts.last().is_some_and(Vec::is_empty) { return Err(invalid(span, "a trailing ; leaves an empty item: [x] is already a one-item vector")); }
        if parts.iter().any(Vec::is_empty) { return Err(invalid(span, "empty item between semicolons")); }
        parts.into_iter().map(expression).collect::<Result<Vec<_>, _>>()?
    } else { items(parts.pop().unwrap())? };
    Ok(NodeKind::ArrayLiteral { record: cells.iter().any(|c| key_colon(c).is_some()), cells, form: ListForm::Items })
}

/// `[…]` or `(…)` with a number marker, whose numbers the lexer has already read as the marker says. Every item must be a literal, and an empty `[]` is the empty vector of the marked numbers.
fn marked_literal(kind: NodeKind, span: &Span, mark: Mark) -> Result<NodeKind, ParseFailure> {
    if matches!(&kind, NodeKind::ArrayLiteral { cells, .. } if cells.is_empty()) { return Ok(NodeKind::Literal(zilde(mark.zero()))); }
    let node = Node { kind, span: span.clone() };
    if let Some(item) = non_literal(std::slice::from_ref(&node), &|_| false) {
        return Err(invalid(&item.span, &format!("{} needs every item to be a literal", mark.marker())));
    }
    Ok(node.kind)
}

/// The first item, at any depth, that isn't a literal, a key's colon or a node that `allowed` accepts. Lists, parentheses and runs are searched through.
pub(crate) fn non_literal<'a>(nodes: &'a [Node], allowed: &impl Fn(&NodeKind) -> bool) -> Option<&'a Node> {
    nodes.iter().find_map(|node| match &node.kind {
        NodeKind::Literal(_) | NodeKind::Function(Builtin::Primitive(Primitive::Keys)) => None,
        NodeKind::ArrayLiteral { cells, .. } => cells.iter().find_map(|cell| non_literal(cell, allowed)),
        NodeKind::Group(nodes) | NodeKind::Run(nodes) => non_literal(nodes, allowed), kind if allowed(kind) => None, _ => Some(node)
    })
}

/// Items separated by spaces: each run of nodes with no space between them is one item.
fn items(nodes: Vec<Node>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let mut runs: Vec<Vec<Node>> = Vec::new();
    for node in nodes { match runs.last_mut() { Some(run) if touching(run.last().unwrap(), &node) => run.push(node), _ => runs.push(vec![node]) } }
    runs.into_iter().map(expression).collect()
}

/// An expression outside brackets, or one item between semicolons: strands, then runs.
fn expression(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let nodes = runs(strands(nodes)?);
    subscripts_follow_items(&nodes)?;
    Ok(nodes)
}

/// Fails unless each subscript follows a node in its list. That node is the item the subscript touches, or after a space the run before it.
fn subscripts_follow_items(nodes: &[Node]) -> Result<(), ParseFailure> {
    if let Some(n @ Node { kind: NodeKind::Subscript(_), .. }) = nodes.first() {
        return Err(invalid(&n.span, "a subscript selects from the item just before it, or after a space from the run before it"));
    }
    for n in nodes { if let NodeKind::Run(inner) = &n.kind { subscripts_follow_items(inner)? } }
    Ok(())
}

/// Literals separated by spaces form a strand, which becomes one vector literal. A literal that touches the one before it starts a new strand: in `⍣3'b'`, `3` is the operand and `'b'` the argument.
fn strands(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let (mut out, mut strand) = (Vec::with_capacity(nodes.len()), Vec::new());
    let mut nodes = nodes.into_iter().peekable();
    while let Some(mut node) = nodes.next() {
        if matches!(node.kind, NodeKind::Literal(_)) {
            // A superscript binds to the one number before it, before literals strand, so `2 3²` is `2 9`.
            while let Some(Node { kind: NodeKind::Operator(OperatorKind::Super(power)), span }) = nodes.peek() {
                let (power, end) = (*power, span.range.end);
                if span.range.start != node.span.range.end { break; }
                let NodeKind::Literal(Value::Number(n)) = &node.kind else { break };
                let raised = match power {
                    Superscript::Power(k) => n.math_dyad(Math::Power, &Number::from_integer(k)), Superscript::Transpose => Ok(n.clone()),
                    Superscript::Unit | Superscript::Floor => break
                };
                node.span.range.end = end;
                let raised = raised.map_err(|m| node.span.error(Kind::Domain, m))?;
                node.kind = NodeKind::Literal(Value::Number(raised));
                nodes.next();
            }
            if strand.last().is_some_and(|last| touching(last, &node)) { close_strand(&mut strand, &mut out)?; }
            strand.push(node);
            continue;
        }
        close_strand(&mut strand, &mut out)?;
        out.push(node);
    }
    close_strand(&mut strand, &mut out)?;
    Ok(out)
}

fn close_strand(strand: &mut Vec<Node>, out: &mut Vec<Node>) -> Result<(), ParseFailure> {
    if strand.len() < 2 { out.append(strand); return Ok(()); }
    let span = cover(strand);
    let items: Vec<_> = strand.drain(..).map(|n| if let NodeKind::Literal(v) = n.kind { v } else { unreachable!() }).collect();
    let value = Value::new([items.len()], items).error_at(&span, "invalid literal list")?;
    out.push(Node { kind: NodeKind::Literal(value), span });
    Ok(())
}

/// Spaces separate runs, and so do error guards. The run holding an assignment's target takes in the `←` and its value, which runs to the next error guard. A target that touches its `←` stays flat within that run, because assignment chooses its own target there.
fn runs(nodes: Vec<Node>) -> Vec<Node> {
    let (mut result, mut part) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if matches!(node.kind, NodeKind::ErrorGuard) {
            result.extend(assignments(std::mem::take(&mut part)));
            result.push(node);
        } else { part.push(node) }
    }
    result.extend(assignments(part));
    result
}

/// Whether `node` is written as a function: a primitive, an operator, a dfn, or parentheses round such nodes.
fn written_function(node: &Node) -> bool {
    match &node.kind { NodeKind::Function(_) | NodeKind::Operator(_) | NodeKind::Dfn(_) => true, NodeKind::Group(inner) => inner.iter().all(written_function), _ => false }
}

/// Assignments bind from the right, so this works back from the last `←`. A chain such as `a←b←0` stays flat. A target that touches its `←` stays flat, because assignment chooses its own target there. A space before `←`, or before a modifier written as functions, as in `x +←1`, makes the whole run before it the target, as if parenthesised.
fn assignments(nodes: Vec<Node>) -> Vec<Node> {
    let mut segments = vec![Vec::new()];
    for node in nodes {
        let assign = matches!(node.kind, NodeKind::Assign);
        segments.last_mut().unwrap().push(node);
        if assign { segments.push(Vec::new()); }
    }
    let mut value: std::collections::VecDeque<Node> = finish(spaced(segments.pop().unwrap())).into();
    while let Some(mut segment) = segments.pop() {
        let assign = segment.pop().unwrap();
        let (mut out, mut target) = spaced(segment);
        let touches = target.last().is_some_and(|n| touching(n, &assign));
        let mut modifier = Vec::new();
        if touches && !out.is_empty() && target.iter().all(written_function) {
            let run = out.pop().unwrap();
            let run = match run.kind { NodeKind::Run(nodes) => Node { kind: NodeKind::Group(nodes), span: run.span }, _ => run };
            modifier = std::mem::replace(&mut target, vec![run]);
        } else if !touches && target.len() > 1 {
            let span = cover(&target);
            target = vec![Node { kind: NodeKind::Group(target), span }];
        }
        value.push_front(assign);
        for node in modifier.into_iter().rev().chain(target.into_iter().rev()) { value.push_front(node); }
        if out.is_empty() { continue; }
        // Runs before the target stay separate from it.
        let nodes = Vec::from(value);
        let span = cover(&nodes);
        out.push(Node { kind: NodeKind::Run(nodes), span });
        value = out.into();
    }
    value.into()
}

/// Nodes with no space between them form runs. Returns the finished runs and the last run, which is still open.
fn spaced(nodes: Vec<Node>) -> (Vec<Node>, Vec<Node>) {
    let (mut out, mut run): (Vec<Node>, Vec<Node>) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if run.last().is_some_and(|last| !touching(last, &node)) { flush(&mut run, &mut out); }
        run.push(node);
    }
    (out, run)
}

fn flush(run: &mut Vec<Node>, out: &mut Vec<Node>) {
    if run.len() < 2 { out.append(run) } else {
        let span = cover(run);
        out.push(Node { kind: NodeKind::Run(std::mem::take(run)), span });
    }
}

/// Close the last run. A run that is the whole expression needs no wrapper.
fn finish((mut out, mut run): (Vec<Node>, Vec<Node>)) -> Vec<Node> {
    flush(&mut run, &mut out);
    if let [Node { kind: NodeKind::Run(inner), .. }] = out.as_mut_slice() { return std::mem::take(inner); }
    out
}

/// The position of the `:` in a bracketed `key:value` item: its first `:`, which follows keys, with no function before them.
pub(crate) fn key_colon(nodes: &[Node]) -> Option<usize> {
    let i = nodes.iter().position(|n| n.kind.primitive() == Some(Primitive::Keys))?;
    (i > 0 && nodes[..i].iter().all(key_node)).then_some(i)
}

/// Whether a node can stand before `:` as a key: a literal, a name, a group or brackets.
pub(crate) fn key_node(n: &Node) -> bool { matches!(n.kind, NodeKind::Literal(_) | NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. }) }

/// The program in `source`, with numbers that have no marker read as floats of the current settings' default width. The current settings also give literals their kinds. They're the session's within an evaluation or `Session::parse`, and the defaults elsewhere, as in a check of structure without evaluation. A complete input can still have a binding or domain error.
pub fn parse(source: Arc<Source>) -> ParseStatus {
    parse_with_aliases(source, false)
}

pub(crate) fn parse_with_aliases(source: Arc<Source>, aliases: bool) -> ParseStatus {
    let tokens = match lex(&source, aliases, None) { Ok(tokens) => tokens, Err(failure) => return failure.into() };
    match (Parser { tokens: &tokens, pos: 0 }).pieces(None).and_then(statements) {
        Ok(statements) => {
            ParseStatus::Complete(Parsed { statements: statements.into_iter().map(|nodes| Statement { nodes, kind: StatementKind::Expression, recording: Default::default() }).collect() })
        }
        Err(failure) => failure.into(),
    }
}

/// A parse tree node for source tools: its kind, its byte range in the source, its children, and the syntax error of an `Error` node.
pub struct SyntaxNode { pub kind: &'static str, pub range: std::ops::Range<usize>, pub children: Vec<SyntaxNode>, pub error: Option<Error> }

impl SyntaxNode {
    fn new(kind: &'static str, range: std::ops::Range<usize>, children: Vec<SyntaxNode>) -> Self { Self { kind, range, children, error: None } }

    fn shifted(mut self, by: usize) -> Self {
        self.range = self.range.start + by..self.range.end + by;
        self.children = self.children.into_iter().map(|c| c.shifted(by)).collect();
        self
    }

    /// Insert `node` under the deepest node whose range holds it, in source order.
    fn insert(&mut self, node: SyntaxNode) {
        if let Some(child) = self.children.iter_mut().find(|c| c.range.start <= node.range.start && node.range.end <= c.range.end) {
            return child.insert(node);
        }
        let i = self.children.partition_point(|c| c.range.start < node.range.start);
        self.children.insert(i, node);
    }
}

/// The parse tree of `text`. Kinds are `NodeKind` variant names, plus `Program`, `Statement`, `Body` for each dfn body, `Cell` for each array-literal cell, `Comment`, and `Error` for lines that don't parse. Separators have no nodes.
pub fn syntax_tree(text: &str) -> SyntaxNode {
    let source = Source::new("<syntax>", text);
    let children = match parse(source.clone()) {
        ParseStatus::Complete(parsed) => parsed.statements.iter().map(statement_node).collect(), _ => recovered(&source)
    };
    let mut tree = SyntaxNode::new("Program", 0..text.len(), children);
    for (range, class) in highlight(text, false) { if class == "comment" { tree.insert(SyntaxNode::new("Comment", range, Vec::new())) } }
    tree
}

/// The statements of `source`, parsed in chunks of whole lines. A chunk grows a line at a time while it's incomplete. A chunk that is invalid, or incomplete at the end of the text, becomes an `Error` node, whose error span is a range of the whole text.
fn recovered(source: &Arc<Source>) -> Vec<SyntaxNode> {
    let text = source.text.as_str();
    let mut ends: Vec<usize> = text.match_indices('\n').map(|(i, _)| i + 1).chain([text.len()]).collect();
    ends.dedup();
    let (mut nodes, mut start) = (Vec::new(), 0);
    while start < text.len() {
        let mut lines = ends.iter().copied().filter(|&end| end > start).peekable();
        let (end, status) = loop {
            let end = lines.next().expect("a line ends after `start`");
            match parse(Source::new("<syntax>", &text[start..end])) {
                ParseStatus::Incomplete(_) if lines.peek().is_some() => continue, status => break (end, status)
            }
        };
        match status {
            ParseStatus::Complete(parsed) => nodes.extend(parsed.statements.iter().map(|s| statement_node(s).shifted(start))),
            ParseStatus::Incomplete(mut error) | ParseStatus::Invalid(mut error) => {
                let range = error.span.range.start + start..error.span.range.end + start;
                error.span = Span { source: source.clone(), range };
                let stop = if text[..end].ends_with('\n') { end - 1 } else { end };
                nodes.push(SyntaxNode { error: Some(error), ..SyntaxNode::new("Error", start..stop, Vec::new()) });
            }
        }
        start = end;
    }
    nodes
}

fn list_node(kind: &'static str, nodes: &[Node]) -> SyntaxNode { SyntaxNode::new(kind, cover(nodes).range, nodes.iter().map(syntax_node).collect()) }

fn statement_node(statement: &Statement) -> SyntaxNode { list_node("Statement", &statement.nodes) }

fn syntax_node(node: &Node) -> SyntaxNode {
    let children = match &node.kind {
        NodeKind::Group(nodes) | NodeKind::Run(nodes) => nodes.iter().map(syntax_node).collect(),
        NodeKind::Scope(target, body) => vec![syntax_node(target), syntax_node(body)],
        NodeKind::ArrayLiteral { cells, .. } => cells.iter().map(|c| list_node("Cell", c)).collect(),
        NodeKind::Dfn(definition) => definition.bodies.iter().map(|body| body_node(body, &definition.span)).collect(), _ => Vec::new()
    };
    SyntaxNode::new(node.kind.name(), node.span.range.clone(), children)
}

/// A dfn body. Only `{}` has an empty body, which spans the text between its braces.
fn body_node(body: &[Statement], dfn: &Span) -> SyntaxNode {
    let range = match (body.first(), body.last()) {
        (Some(first), Some(last)) => cover(&first.nodes).range.start..cover(&last.nodes).range.end, _ => dfn.range.start + 1..dfn.range.end - 1
    };
    SyntaxNode::new("Body", range, body.iter().map(statement_node).collect())
}

impl NodeKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Literal(_) => "Literal", Self::Function(_) => "Function", Self::Operator(_) => "Operator", Self::Name(_) => "Name",
            Self::System(_) => "System", Self::Assign => "Assign", Self::Output => "Output", Self::ErrorGuard => "ErrorGuard", Self::Group(_) => "Group",
            Self::Run(_) => "Run", Self::Subscript(_) => "Subscript", Self::Scope(..) => "Scope", Self::ArrayLiteral { .. } => "ArrayLiteral",
            Self::Dfn(_) => "Dfn", Self::Joiner(_) => "Joiner"
        }
    }
    /// The primitive that a glyph's node holds, or `None` for any other node.
    pub(crate) fn primitive(&self) -> Option<Primitive> { match self { Self::Function(Builtin::Primitive(p)) => Some(*p), _ => None } }
}
