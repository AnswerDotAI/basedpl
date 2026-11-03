use crate::name::{Name, is_implicit};
use crate::{
    At, Described, Error, ErrorKind, Number, Source, Span, Value, eval::OperatorKind,
    number::{Arithmetic, DENOMINATOR, EXPONENT, IMAGINARY, MARKS, Mark, Math, SUFFIXES, Suffix, subscript}, primitive::{Builtin, Exact, Primitive},
    symbols::canonical
};
use rustymath::{buffer::{DType, FloatWidth}, error::Kind};
use std::{iter::Peekable, str::CharIndices, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Literal(Value),
    Number { text: Option<&'static str>, mark: Option<Mark> },
    Zilde(Option<Mark>),
    BoundOperator(OperatorKind, Value),
    Function(Builtin),
    Operator(OperatorKind),
    /// `ᵀ` or `ᵘ` (`Letter`).
    Letter(Letter),
    /// A name. `true` marks a read of a dfn's local, `⍵` or `⍺` after which nothing in the dfn reads or assigns the name (`release`). That read moves the value out of the frame.
    Name(Name, bool),
    /// A system name that the system table doesn't hold, which is a VALUE error when evaluated. A known system name is a `Function`, as a glyph is.
    System(String),
    Assign,
    Output,
    ErrorGuard,
    Group(Vec<Node>),
    /// A run of nodes with no spaces between them, evaluated before its neighbours.
    Run(Vec<Node>),
    /// Numeric subscripts select from an array or set a function's axes, after dot access.
    Subscript(Value),
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

/// A letter superscript (target part 21): `ᵀ` after an array transposes it, and `ᵘ` after a function makes a scalar holding it. Neither is an operator: the binder compiles each by the role of the item before it (`tree::Postfix`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Letter { Transpose, Unit }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefinitionKind { Function, MonadicOperator, DyadicOperator }

#[derive(Debug)]
pub(crate) struct Definition {
    pub bodies: Vec<Vec<Statement>>, pub span: Span, pub kind: DefinitionKind, locals: &'static [Name], acts: bool, implicit_generator: bool,
    reads: &'static [Name]
}
impl Definition {
    pub(crate) fn new(mut bodies: Vec<Vec<Statement>>, span: Span) -> Self {
        let kind = definition_kind(&bodies);
        let locals = locals(&bodies);
        let (acts, implicit_generator) = effects(&bodies);
        let reads = read_names(&bodies);
        release(&mut bodies, locals);
        Self { bodies, span, kind, locals, acts, implicit_generator, reads }
    }
    /// The names that the definition could assign, sorted, which each of its calls keeps in slots.
    pub fn locals(&self) -> &'static [Name] { self.locals }
    /// Effects visible in its syntax, including nested definitions.
    pub(crate) fn acts(&self) -> bool { self.acts }
    pub(crate) fn implicit_generator(&self) -> bool { self.implicit_generator }
    pub(crate) fn reads(&self) -> &'static [Name] { self.reads }
}

fn definition_kind(bodies: &[Vec<Statement>]) -> DefinitionKind {
    let mut kind = DefinitionKind::Function;
    for statement in bodies.iter().flatten() {
        // Nested definitions classify their own bodies.
        visit(&statement.nodes, NameUse::default(), false, &mut |node, _, nested| {
            kind = kind.max(match &node.kind {
                NodeKind::Name(Name::RIGHT_OPERAND, _) if !nested => DefinitionKind::DyadicOperator,
                NodeKind::Name(Name::LEFT_OPERAND, _) if !nested => DefinitionKind::MonadicOperator, _ => DefinitionKind::Function
            })
        });
    }
    kind
}

/// The names that a definition's statements could assign, sorted: each name in a target outside nested definitions (`visit`). A call gives each a slot. Some name a modified assignment's function, or the array that a selective assignment selects from, and their slots stay empty.
fn locals(bodies: &[Vec<Statement>]) -> &'static [Name] {
    let mut names = Vec::new();
    for statement in bodies.iter().flatten() {
        visit(&statement.nodes, NameUse::default(), false, &mut |node, target, nested| {
            if let NodeKind::Name(name, _) = &node.kind && target.binding && !nested { names.push(*name) }
        });
    }
    names.retain(|name| name.implicit().is_none());
    names.sort_unstable();
    names.dedup();
    crate::name::kept(&names)
}

/// All lexical reads, including locals whose empty slots fall through to an outer binding.
fn read_names(bodies: &[Vec<Statement>]) -> &'static [Name] {
    let mut names = Vec::new();
    for statement in bodies.iter().flatten() {
        visit(&statement.nodes, NameUse::default(), false, &mut |node, target, nested| {
            if nested { return; }
            match &node.kind {
                NodeKind::Name(name, _) if !target.binding && name.implicit().is_none() => names.push(*name),
                NodeKind::Dfn(definition) => names.extend_from_slice(definition.reads()), _ => {}
            }
        });
    }
    names.sort_unstable();
    names.dedup();
    crate::name::kept(&names)
}
fn effects(bodies: &[Vec<Statement>]) -> (bool, bool) {
    let (mut acts, mut implicit_generator) = (false, false);
    for statement in bodies.iter().flatten() {
        visit(&statement.nodes, NameUse::default(), false, &mut |node, _, nested| {
            if nested { return; }
            match &node.kind {
                NodeKind::Function(Builtin::Primitive(primitive)) if primitive.implicit_generator() => implicit_generator = true,
                NodeKind::Output | NodeKind::Function(Builtin::Primitive(Primitive::Execute)) => acts = true,
                NodeKind::Function(Builtin::System(entry)) => {
                    let function = entry.built();
                    acts |= function.system_function().is_some_and(|function| function.acts());
                    implicit_generator |= function.implicit_generator();
                }
                NodeKind::Dfn(definition) => {
                    acts |= definition.acts;
                    implicit_generator |= definition.implicit_generator;
                }
                _ => {}
            }
        });
    }
    (acts, implicit_generator)
}

pub(crate) fn plain_target(nodes: &[Node]) -> bool {
    let [node] = nodes else { return false };
    match &node.kind {
        NodeKind::Name(..) => true,
        NodeKind::Group(inner) => plain_target(inner),
        NodeKind::ArrayLiteral { cells, form: ListForm::Items | ListForm::Rows, record } => {
            cells.iter().all(|cell| if *record { key_colon(cell).is_some_and(|colon| plain_target(&cell[colon + 1..])) } else { plain_target(cell) })
        }
        _ => false,
    }
}
#[derive(Clone, Copy, Default)]
struct NameUse { binding: bool, write: bool }
fn visit(nodes: &[Node], parent: NameUse, nested: bool, f: &mut dyn FnMut(&Node, NameUse, bool)) {
    let end = nodes.iter().rposition(|node| matches!(node.kind, NodeKind::Assign)).unwrap_or(0);
    let binds = plain_target(&nodes[..end]);
    for (i, node) in nodes.iter().enumerate() {
        let target = NameUse { binding: parent.binding || binds && i < end, write: parent.write || i < end };
        f(node, target, nested);
        match &node.kind {
            NodeKind::Group(nodes) | NodeKind::Run(nodes) => visit(nodes, target, nested, f),
            NodeKind::ArrayLiteral { cells, .. } => { for nodes in cells { visit(nodes, target, nested, f); } }
            NodeKind::Scope(record, body) => {
                visit(std::slice::from_ref(record), target, nested, f);
                visit(std::slice::from_ref(body), target, nested, f);
            }
            NodeKind::Dfn(definition) => { for statement in definition.bodies.iter().flatten() { visit(&statement.nodes, NameUse::default(), true, f); } }
            _ => {}
        }
    }
}

/// A last read moves its binding out of the frame. Nested dfns, error handlers and text-based lookups retain the bindings they can read.
fn release(bodies: &mut [Vec<Statement>], locals: &[Name]) {
    let mut by_text = false;
    // The names that a nested definition, an error handler or a later statement mentions.
    let mut later = Vec::new();
    for statement in bodies.iter().flatten() {
        visit(&statement.nodes, NameUse::default(), false, &mut |node, _, nested| match &node.kind {
            NodeKind::Function(Builtin::Primitive(Primitive::Execute)) => by_text = true,
            NodeKind::Function(Builtin::System(entry)) => by_text |= matches!(entry.name(), "•nc" | "•nl" | "•src" | "•ex" | "•explain"),
            NodeKind::Name(name, _) if nested => later.push(*name), _ => {}
        });
        if let StatementKind::ErrorGuard { index } = statement.kind {
            visit(&statement.nodes[index + 1..], NameUse::default(), false, &mut |node, _, _| {
                if let NodeKind::Name(name, _) = &node.kind { later.push(*name) }
            });
        }
    }
    if by_text { return; }
    for statement in bodies.iter_mut().flatten().rev() {
        let (mut reads, mut targets) = (Vec::new(), Vec::new());
        visit(&statement.nodes, NameUse::default(), false, &mut |node, target, nested| {
            let NodeKind::Name(name, _) = &node.kind else { return };
            if nested { return; }
            if target.write { targets.push(*name); }
            if !target.binding && !reads.contains(name) { reads.push(*name); }
        });
        let releases = |name: &Name| locals.binary_search(name).is_ok() || matches!(*name, Name::OMEGA | Name::ALPHA);
        for &name in reads.iter().filter(|&name| releases(name) && !targets.contains(name) && !later.contains(name)) {
            let _ = mark_last(&mut statement.nodes, name);
        }
        later.extend(reads.into_iter().chain(targets));
    }
}

/// Marks the read of `name` in `nodes` that runs last, and says whether `nodes` read it at all, or gives `Err` and marks nothing where the order isn't known. A node list runs right to left, so that read is in the leftmost node that reads the name. A bracket's items run left to right, so within a bracket it's in the rightmost item that reads the name. Within a term that a dot, a subscript or a joiner joins, the root evaluates first where it holds an array (`Session::members_with`), and is an inner product's operand where it's a function. Only binding knows which, so a term that reads the name twice gives `Err`.
fn mark_last(nodes: &mut [Node], name: Name) -> Result<bool, ()> {
    let reads = |node: &Node| {
        let mut found = false;
        visit(std::slice::from_ref(node), NameUse::default(), false, &mut |n, _, _| found |= matches!(n.kind, NodeKind::Name(m, _) if m == name));
        found
    };
    let Some(i) = nodes.iter().position(reads) else { return Ok(false) };
    let joins = |kind: &NodeKind| matches!(kind, NodeKind::Joiner(_) | NodeKind::Operator(OperatorKind::Dot));
    let end = (i + 1..nodes.len()).find(|&j| !matches!(nodes[j].kind, NodeKind::Subscript(_)) && !joins(&nodes[j].kind) && !joins(&nodes[j - 1].kind))
        .unwrap_or(nodes.len());
    if nodes[i + 1..end].iter().any(reads) { return Err(()); }
    match &mut nodes[i].kind {
        NodeKind::Name(_, last) => { *last = true; Ok(true) } NodeKind::Group(nodes) | NodeKind::Run(nodes) => mark_last(nodes, name),
        NodeKind::ArrayLiteral { cells, .. } => { for cell in cells.iter_mut().rev() { if mark_last(cell, name)? { return Ok(true); } } Ok(false) }
        _ => Ok(false)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Node { pub kind: NodeKind, pub span: Span }

/// Structural syntax, not an arithmetic AST. Name roles will be resolved during binding.
#[derive(Clone, Debug)]
pub struct Parsed { pub(crate) statements: Vec<Statement> }

#[derive(Clone, Debug)]
/// A statement's syntax. Its binder instructions are local to each execution.
pub(crate) struct Statement { pub nodes: Vec<Node>, pub kind: StatementKind }

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
    } else if matches!(nodes[0].kind, NodeKind::Name(Name::ALPHA, _)) && matches!(nodes.get(1).map(|n| &n.kind), Some(NodeKind::Assign)) {
        StatementKind::DefaultArgument
    } else { StatementKind::Expression };
    Ok(Statement { nodes, kind })
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
    Number(Option<Mark>),
    Zilde(Option<Mark>),
    Power(i64),
    Function(Primitive),
    Operator(OperatorKind),
    /// `ᵀ` or `ᵘ` (`Letter`).
    Letter(Letter),
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
impl<'a> std::ops::Deref for Characters<'a> {
    type Target = Peekable<CharIndices<'a>>;
    fn deref(&self) -> &Self::Target { &self.chars }
}
impl std::ops::DerefMut for Characters<'_> { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.chars } }
impl Iterator for Characters<'_> {
    type Item = (usize, char);
    fn next(&mut self) -> Option<Self::Item> { self.chars.next() }
}

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

/// `ⱼ`, `ₑ` or `ᵣ` between two items, as in `xⱼy`. The binder joins the items into one term (`joined_term`).
#[derive(Clone, Copy, Debug)]
pub(crate) enum Joiner { Imaginary, Exponent, Denominator }

/// The group that `left`, `joiner` at `at` and `right` stand for: `xⱼy` is `x+y×0ⱼ1`, `xₑy` is `x×10*y` and `xᵣy` forms a fraction from `⌊xᵣ` and `⌊yᵣ` with a nonzero denominator.
pub(crate) fn joined_term(left: Node, joiner: Joiner, at: &Span, right: Node) -> Node {
    let span = Span { source: at.source.clone(), range: left.span.range.start..right.span.range.end };
    let node = |kind| Node { kind, span: at.clone() };
    let number = |text| node(NodeKind::Number { text: Some(text), mark: None });
    let function = |p| node(NodeKind::Function(Builtin::Primitive(p)));
    let nodes = match joiner {
        Joiner::Imaginary => {
            vec![left, function(Primitive::Arithmetic(Arithmetic::Plus)), right, function(Primitive::Arithmetic(Arithmetic::Times)), number("0j1")]
        }
        Joiner::Exponent => vec![left, function(Primitive::Arithmetic(Arithmetic::Times)), number("10"), function(Primitive::Math(Math::Power)), right],
        Joiner::Denominator => {
            let floor = |value| {
                node(NodeKind::Group(vec![function(Primitive::Math(Math::Floor)), value, node(NodeKind::Operator(OperatorKind::Mark(Mark::Rational)))]))
            };
            vec![floor(left), function(Primitive::Exact(Exact::Ratio)), floor(right)]
        }
    };
    Node { kind: NodeKind::Group(nodes), span }
}

/// The single-character mark at the next position.
fn mark_head(chars: &mut Characters<'_>) -> Option<Mark> { let &(_, c) = chars.peek()?; MARKS.iter().find_map(|&(s, m)| s.chars().eq([c]).then_some(m)) }

fn mark(chars: &mut Characters<'_>) -> Result<Option<Mark>, (ErrorKind, String)> {
    let Some(mark) = mark_head(chars) else { return Ok(None) };
    chars.next();
    if mark != Mark::Exact { return Ok(Some(mark)); }
    let Some(&(_, c)) = chars.peek().filter(|(_, c)| *c == '₋' || script_digit(SUBSCRIPT_DIGITS, *c).is_some()) else { return Ok(Some(mark)) };
    chars.next();
    let bytes = script_integer(c, chars, SUBSCRIPT_DIGITS, "integer width")?;
    MARKS.iter().find_map(|&(spelling, m)| match m {
        Mark::Type { dtype: DType::Integer(w), fixed: true } if spelling.ends_with(c) && DType::Integer(w).bytes() as i64 == bytes => Some(Some(m)), _ => None
    }).ok_or_else(|| (ErrorKind::Syntax, "fixed integer width must be 1, 2, 4 or 8 bytes".into()))
}

/// Consumes a float width's marker when one comes next, and gives the width.
fn width_mark(chars: &mut Characters<'_>) -> Option<FloatWidth> {
    let Mark::Type { dtype: DType::Float(width), .. } = mark_head(chars)? else { return None };
    chars.next();
    Some(width)
}

/// Consumes `ₐ` when it comes next. A rational or complex literal can end in it, which `Number::parse` reads as a conversion to a real float.
fn approximate_mark(chars: &mut Characters<'_>) { if mark_head(chars) == Some(Mark::Real) { chars.next(); } }

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

/// The empty numeric vector whose prototype is `fill`.
pub(crate) fn zilde(fill: Number) -> Value { Value::empty([0], Value::Number(fill)).unwrap() }

/// Whether a token can be part of a marked literal: a literal, a parenthesis or bracket, `;`, `⋄`, a key's colon, or a superscript that the parser folds into the number before it.
fn literal_token(kind: &TokenKind) -> bool {
    matches!(
        kind, TokenKind::Literal(_) | TokenKind::Number(_) | TokenKind::Zilde(_) | TokenKind::Open | TokenKind::Close(_) | TokenKind::BracketOpen
            | TokenKind::BracketClose(_) | TokenKind::Semicolon | TokenKind::Separator | TokenKind::Function(Primitive::Keys(false)) | TokenKind::Power(_)
            | TokenKind::Letter(Letter::Transpose)
    )
}

/// Attaches literal markers to syntax; number construction happens in the executor.
fn mark_literals(tokens: &mut Vec<Token>) -> Result<Vec<std::ops::Range<usize>>, Error> {
    let (mut open, mut marked, mut markers) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..tokens.len() {
        match tokens[i].kind {
            TokenKind::Open | TokenKind::BracketOpen => open.push(i),
            TokenKind::Close(mark) | TokenKind::BracketClose(mark) => {
                let Some(start) = open.pop() else { continue };
                let Some(mark) = mark else { continue };
                if !tokens[start + 1..i].iter().all(|token| literal_token(&token.kind)) { marked.push((i, mark)); continue; }
                for token in &mut tokens[start + 1..i] {
                    match token.kind {
                        TokenKind::Number(_) | TokenKind::Literal(Value::Number(_)) => token.kind = TokenKind::Number(Some(mark)),
                        TokenKind::Zilde(_) => token.kind = TokenKind::Zilde(Some(mark)), _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    for (i, mark) in marked.into_iter().rev() {
        let close = &mut tokens[i];
        close.kind = match close.kind { TokenKind::Close(_) => TokenKind::Close(None), _ => TokenKind::BracketClose(None) };
        let end = close.span.range.end;
        close.span.range.end = end - mark.marker().len();
        let marker = Span { source: close.span.source.clone(), range: close.span.range.end..end };
        markers.push(marker.range.clone());
        tokens.insert(i + 1, Token { kind: TokenKind::Operator(OperatorKind::Mark(mark)), span: marker });
    }
    Ok(markers)
}

/// The offset of the next character, or the end of the text.
fn position(chars: &mut Characters<'_>, len: usize) -> usize { chars.peek().map_or(len, |&(i, _)| i) }

/// The end of a character that would run on from a numeric literal: a point, a numeric suffix, or a digit when `digit` is set.
fn run_on(chars: &mut Characters<'_>, digit: bool) -> Option<usize> {
    let canonical = chars.spelling();
    let &(i, c) = chars.peek()?;
    let suffix = mark_head(chars).is_some() || SUFFIXES.into_iter().any(|s| suffix_follows(chars, s));
    ((digit && c.is_ascii_digit()) || canonical(c) == '.' || suffix).then(|| i + c.len_utf8())
}

/// Whether `c` can start or continue a name. Syntax, glyphs and digits end a name.
pub(crate) fn name_char(c: char) -> bool {
    !c.is_whitespace() && !c.is_control() && !c.is_ascii_digit() && !subscript(c) && !matches!(
        c, '(' | ')' | '[' | ']' | '{' | '}' | '\'' | '"' | ';' | ':' | '?' | '$' | '•' | '←' | '⎕' | '⍝' | '⋄' | '⁝' | '⍬' | '¯' | '∞' | '⍛'
            | 'ᵀ' | 'ᵘ' | '⁻' | '₋'
    ) && !is_implicit(c) && script_digit(SUPERSCRIPT_DIGITS, c).is_none() && script_digit(SUBSCRIPT_DIGITS, c).is_none() && Primitive::from_glyph(c).is_none()
        && OperatorKind::from_glyph(c).is_none()
}

pub(crate) fn name_char_in(c: char, aliases: bool) -> bool { name_char(if aliases { canonical(c) } else { c }) }

const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
const SUBSCRIPT_DIGITS: &str = "₀₁₂₃₄₅₆₇₈₉";
/// The characters that write a superscript operator, `ₓ` and `ₐ` among them, for the editors' glyph lists.
pub fn superscripts() -> String { [SUPERSCRIPT_DIGITS, "⁻ᵀᵘₓₐᵣ"].concat() }
/// The characters that write a subscript, for the editors' glyph lists.
pub fn subscripts() -> String { [SUBSCRIPT_DIGITS, "₋"].concat() }

/// Number markers and their word names, for Python properties and editor generation.
pub fn number_marks() -> Vec<(&'static str, &'static str)> {
    MARKS.iter().map(|&(text, mark)| {
        (text, match mark { Mark::Exact => "exact", Mark::Rational => "rational", Mark::Type { dtype, .. } => dtype.name(), Mark::Real => "approximate" })
    }).collect()
}

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
                match chars.next() { Some((_, q)) if !q.is_whitespace() => { if q.is_ascii_digit() { digits(chars); } } _ => return Err((i + 1, UNQUOTED)) }
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
        [Node { kind: NodeKind::Function(Builtin::System(crate::system::Entry::PREFS)), .. },
            record @ Node { kind: NodeKind::ArrayLiteral { record: true, .. }, .. },
        ] if non_literal(std::slice::from_ref(record), &|_| false).is_none() => Some(record), _ => None
    }
}

/// The header only changes lexing. Numeric preferences execute as an ordinary statement.
fn header(tokens: &[Token], aliases: &mut bool) -> bool {
    let Some(start) = tokens.iter().position(|token| !matches!(token.kind, TokenKind::Separator)) else { return false; };
    let tokens = &tokens[start..];
    if !matches!(tokens.first().map(|token| &token.kind), Some(TokenKind::System(name)) if crate::system::Entry::find(name) == Some(crate::system::Entry::PREFS))
    { return false; }
    let Ok(statements) = (Parser { tokens, pos: 0 }).pieces(None).and_then(statements) else { return false; };
    let [nodes] = statements.as_slice() else { return false; };
    let Some(record) = prefs_record(nodes) else { return false; };
    let NodeKind::ArrayLiteral { cells, .. } = &record.kind else { unreachable!() };
    for cell in cells {
        let [Node { kind: NodeKind::Literal(key), .. }, colon, value] = cell.as_slice() else { continue; };
        if colon.kind.primitive() != Some(Primitive::Keys(false)) || crate::keyed::name(key).as_deref() != Some("hanzi_aliases") { continue; }
        let on = match &value.kind {
            NodeKind::Literal(value) => value.boolean().ok(),
            NodeKind::Number { text: None, mark: None } => match value.span.text() { "0" => Some(false), "1" => Some(true), _ => None }, _ => None
        };
        if let Some(on) = on { *aliases = on; }
    }
    true
}

type Highlights = Vec<(std::ops::Range<usize>, &'static str)>;

/// The tokens of `source`. Each leading `•prefs` statement on a literal record sets the hanzi aliases and the numeric settings that the tokens after it read under, as `header` applies them.
fn lex(source: &Arc<Source>, aliases: bool, highlights: Option<&mut Highlights>) -> Result<Vec<Token>, ParseFailure> { lexed(source, aliases, highlights) }

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
                let end = e.span.as_ref().map_or(start, |s| s.range.end).max(start + c.len_utf8());
                while chars.next_if(|&(i, _)| i < end).is_some() {}
                if matches!(c, '"' | '\'') { classes.push((start..position(&mut chars, len), "string")); }
                continue;
            }
        };
        if let Some(classes) = highlights.as_deref_mut() {
            let class = match &kind {
                Some(kind) => class(kind, &source.text[start..position(&mut chars, len)], canonical),
                None => match canonical(c) { '⍝' => Some("comment"), '⁝' => Some("syntax"), _ => None },
            };
            if let Some(class) = class { classes.push((start..position(&mut chars, len), class)); }
            // A group's mark is part of its literal, unless `mark_literals` finds that the group holds anything else.
            if let Some(TokenKind::Close(Some(_)) | TokenKind::BracketClose(Some(_))) = &kind {
                classes.push((start + c.len_utf8()..position(&mut chars, len), "literal"));
            }
        }
        let Some(kind) = kind else { continue };
        if let Some(at) = line_break.take().filter(|_| !absorbs_line_break(&kind, false)) { tokens.push(Token { kind: TokenKind::Separator, span: at }); }
        match kind {
            TokenKind::Open | TokenKind::BracketOpen | TokenKind::BraceOpen => depth += 1,
            TokenKind::Close(_) | TokenKind::BracketClose(_) | TokenKind::BraceClose => depth = depth.saturating_sub(1), _ => ()
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
    let len = source.text.len();
    let Some(&(start, raw)) = chars.peek() else { return Ok(None) };
    let c = canonical(raw);
    let span = |end| Span { source: source.clone(), range: start..end };
    let marked =
        |chars: &mut Characters<'_>| mark(chars).map_err(|(kind, message)| ParseFailure::Invalid(Error::new(kind, message).at(&span(position(chars, len)))));
    Ok(Some(if c.is_ascii_digit() || matches!(c, '¯' | '∞') || (c == '.' && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit())) {
        real_literal(chars).map_err(|message| Error::new(ErrorKind::Syntax, message).at(&span(position(chars, len))))?;
        if suffix(chars, IMAGINARY) {
            real_literal(chars).map_err(|message| Error::new(ErrorKind::Syntax, message).at(&span(position(chars, len))))?;
            if width_mark(chars).is_none() { approximate_mark(chars); }
            if let Some(end) = run_on(chars, false) { return Err(Error::new(ErrorKind::Syntax, "invalid complex numeric literal").at(&span(end)).into()); }
        } else {
            let rational = suffix(chars, DENOMINATOR);
            if rational {
                chars.next_if(|&(_, c)| canonical(c) == '¯');
                if digits(chars) == 0 { return Err(Error::new(ErrorKind::Syntax, "expected integer denominator").at(&span(position(chars, len))).into()); }
                approximate_mark(chars);
            }
            if rational || marked(chars)?.is_some() {
                if let Some(end) = run_on(chars, true) { return Err(Error::new(ErrorKind::Syntax, "invalid exact numeric literal").at(&span(end)).into()); }
            }
        }
        let end = position(chars, len);
        TokenKind::Number(None)
    } else if name_char_in(raw, chars.aliases) {
        let key = source.text[..start].chars().next_back().is_some_and(|c| canonical(c) == '.');
        name(chars, key).map_err(|(end, message)| Error::new(ErrorKind::Syntax, message).at(&span(end)))?;
        TokenKind::Name(source.text[start..position(chars, len)].to_owned())
    } else if c == 'ᵣ' { chars.next(); TokenKind::Joiner(Joiner::Denominator) }
    else if let Some(mark) = marked(chars)? { TokenKind::Operator(OperatorKind::Mark(mark)) } else {
        chars.next();
        match c {
            '\'' => {
                let unclosed = |at| Err(Error::new(ErrorKind::Syntax, "unclosed character literal").at(&span(at)).into());
                let c = match chars.next() { Some((_, c)) if c != '\n' => c, next => return unclosed(next.map_or(len, |(i, _)| i)) };
                if c == '\'' && chars.peek().is_none_or(|&(_, c)| c != '\'') { TokenKind::Literal(crate::keyed::text("")) } else {
                    match chars.peek() {
                        Some((_, '\'')) => { chars.next(); }
                        None | Some((_, '\n')) => return unclosed(position(chars, len)),
                        Some(&(i, c)) => {
                            return Err(Error::new(ErrorKind::Syntax, "a character literal holds one character (strings use double quotes)")
                                .at(&span(i + c.len_utf8())).into());
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
                        None => return Err(ParseFailure::Incomplete(Error::new(ErrorKind::Syntax, "unclosed string").at(&span(len)))),
                    }
                }
                TokenKind::Literal(crate::keyed::text(&text))
            }
            '⍬' => TokenKind::Zilde(marked(chars)?),
            // `$` and one letter: a literal constant, or `$e`, the error a handler caught. `$n` and `$m` take a width mark.
            '$' => {
                let token = match chars.next() {
                    Some((_, 't')) => TokenKind::Literal(Value::Number(Number::from_bool(true))),
                    Some((_, 'f')) => TokenKind::Literal(Value::Number(Number::from_bool(false))),
                    Some((_, 'n')) => { width_mark(chars); TokenKind::Number(None) }
                    Some((_, 'a')) => TokenKind::Literal(crate::keyed::text("ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
                    Some((_, 'd')) => TokenKind::Literal(crate::keyed::text("0123456789")), Some((_, 'm')) => { width_mark(chars); TokenKind::Number(None) }
                    Some((_, 'e')) => TokenKind::Name("$e".into()),
                    _ => return Err(Error::new(ErrorKind::Syntax, DOLLAR).at(&span(position(chars, len))).into())
                };
                if chars.peek().is_some_and(|&(_, c)| name_char(c) || c.is_ascii_digit()) {
                    return Err(Error::new(ErrorKind::Syntax, DOLLAR).at(&span(position(chars, len))).into());
                }
                token
            }
            '⍛' => {
                return Err(Error::new(ErrorKind::Syntax, "⍛ is retired: use ↣ or ↢ to bind or preprocess, and ∘ for Atop")
                    .at(&span(start + c.len_utf8())).into());
            }
            '(' => TokenKind::Open,
            ')' => TokenKind::Close(marked(chars)?),
            '[' => TokenKind::BracketOpen,
            ']' => TokenKind::BracketClose(marked(chars)?),
            ';' => TokenKind::Semicolon,
            '⋄' => TokenKind::Separator,
            '←' => TokenKind::Assign,
            '•' if chars.peek().is_some_and(|&(_, c)| name_char(c)) => {
                name(chars, false).map_err(|(end, message)| Error::new(ErrorKind::Syntax, message).at(&span(end)))?;
                TokenKind::System(format!("•{}", &source.text[start + raw.len_utf8()..position(chars, len)]))
            }
            '⎕' => TokenKind::Output,
            ':' if chars.peek().is_some_and(|(_, c)| canonical(*c) == ':') => { chars.next(); TokenKind::ErrorGuard }
            ':' => TokenKind::Function(Primitive::Keys(false)),
            '?' => TokenKind::Predicate,
            '{' => TokenKind::BraceOpen,
            '}' => TokenKind::BraceClose,
            c if is_implicit(c) => TokenKind::Name(c.to_string()),
            '⍝' => { while chars.peek().is_some_and(|(_, c)| *c != '\n') { chars.next(); } return Ok(None); }
            'ᵀ' => TokenKind::Letter(Letter::Transpose),
            'ᵘ' => TokenKind::Letter(Letter::Unit),
            // A literal number takes `ⱼ`, `ₑ` or `ᵣ` only before a number part (`suffix_follows`). One that reaches here joins two items, as in `xⱼy`.
            'ⱼ' => TokenKind::Joiner(Joiner::Imaginary),
            'ₑ' => TokenKind::Joiner(Joiner::Exponent),
            c if c == '⁻' || script_digit(SUPERSCRIPT_DIGITS, c).is_some() => {
                let power = script_integer(c, chars, SUPERSCRIPT_DIGITS, "superscript").map_err(|(k, m)| Error::new(k, m).at(&span(position(chars, len))))?;
                TokenKind::Power(power)
            }
            c if c == '₋' || script_digit(SUBSCRIPT_DIGITS, c).is_some() => TokenKind::Subscript(
                script_integer(c, chars, SUBSCRIPT_DIGITS, "subscript").map_err(|(k, m)| Error::new(k, m).at(&span(position(chars, len))))?),
            c if c.is_whitespace() || c == '⁝' => return Ok(None),
            c => match (OperatorKind::from_glyph(c), Primitive::from_glyph(c)) {
                (Some(op), _) => TokenKind::Operator(op), (_, Some(f)) => TokenKind::Function(f),
                _ => return Err(Error::new(ErrorKind::Unsupported, format!("{raw:?} is not supported yet")).at(&span(start + raw.len_utf8())).into())
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
        TokenKind::Number(_) | TokenKind::Zilde(_) => Some("literal"),
        TokenKind::Power(_) | TokenKind::Operator(OperatorKind::Mark(_)) | TokenKind::Letter(_) => Some("monadic-operator"),
        TokenKind::Subscript(_) | TokenKind::ErrorGuard => Some("syntax"), TokenKind::Joiner(_) => Some("literal"), TokenKind::Name(_) => Some("name"),
        TokenKind::System(_) => Some("function"), _ => None
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

fn invalid(span: &Span, message: &str) -> ParseFailure { ParseFailure::Invalid(Error::new(ErrorKind::Syntax, message).at(span)) }
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
                            (&o.kind, &token.kind), (TokenKind::Open, TokenKind::Close(_)) | (TokenKind::BracketOpen, TokenKind::BracketClose(_))
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
                        TokenKind::BraceOpen => NodeKind::Dfn(Arc::new(Definition::new(bodies(inner, &span)?, span.clone()))),
                        TokenKind::Open if let Some(mark) = mark => marked_literal(parenthesised(inner, &span)?, &span, mark)?,
                        TokenKind::Open => parenthesised(inner, &span)?, _ if let Some(mark) = mark => marked_literal(brackets(inner, &span)?, &span, mark)?,
                        _ => brackets(inner, &span)?
                    }
                }
                TokenKind::Literal(a) => NodeKind::Literal(a.clone()),
                TokenKind::Number(mark) => NodeKind::Number { text: None, mark: *mark },
                TokenKind::Zilde(mark) => NodeKind::Zilde(*mark),
                TokenKind::Power(-1) => NodeKind::Operator(OperatorKind::Inverse),
                TokenKind::Power(power) => NodeKind::BoundOperator(OperatorKind::Power, Value::integer(*power)),
                TokenKind::Function(f) => NodeKind::Function(Builtin::Primitive(*f)),
                TokenKind::Operator(op) => NodeKind::Operator(op.clone()),
                TokenKind::Letter(letter) => NodeKind::Letter(*letter),
                TokenKind::Subscript(index) => {
                    let first = Value::Number(Number::from_integer(*index));
                    let mut indices = Vec::new();
                    while let Some(Token { kind: TokenKind::Subscript(index), span: next }) = self.tokens.get(self.pos) {
                        let gap = &span.source.text[span.range.end..next.range.start];
                        if gap.is_empty() || !gap.chars().all(char::is_whitespace) { break; }
                        if indices.is_empty() { indices.push(first.clone()); }
                        indices.push(Value::Number(Number::from_integer(*index)));
                        span.range.end = next.range.end;
                        self.pos += 1;
                    }
                    NodeKind::Subscript(if indices.is_empty() { first }
                    else { Value::new([indices.len()], indices).described("invalid subscript list").at(&span)? })
                }
                TokenKind::Joiner(joiner) => NodeKind::Joiner(*joiner),
                TokenKind::Name(name) => NodeKind::Name(Name::new(name), false),
                TokenKind::System(name) => Builtin::find(name).map_or_else(|| NodeKind::System(name.clone()), NodeKind::Function),
                TokenKind::Assign => NodeKind::Assign,
                TokenKind::Output => NodeKind::Output,
                TokenKind::ErrorGuard => {
                    if !open.is_some_and(|o| matches!(o.kind, TokenKind::BraceOpen)) { return Err(invalid(&token.span, "error guards belong to dfns")); }
                    NodeKind::ErrorGuard
                }
            };
            pieces.push(Piece::Node(Node { kind, span }));
        }
        if let Some(open) = open { return Err(ParseFailure::Incomplete(Error::new(ErrorKind::Syntax, "unclosed delimiter").at(&open.span))); }
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

/// Whether only token separators, if anything, come between two nodes in the source.
pub(crate) fn touching(a: &Node, b: &Node) -> bool {
    a.span.source.text.get(a.span.range.end..b.span.range.start).is_some_and(|gap| gap.chars().all(|c| canonical(c) == '⁝'))
}

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
    if matches!(&kind, NodeKind::ArrayLiteral { cells, .. } if cells.is_empty()) { return Ok(NodeKind::Zilde(Some(mark))); }
    let node = Node { kind, span: span.clone() };
    if let Some(item) = non_literal(std::slice::from_ref(&node), &|_| false) {
        return Err(invalid(&item.span, &format!("{} needs every item to be a literal", mark.marker())));
    }
    Ok(node.kind)
}

/// The first item, at any depth, that isn't a literal, a key's colon or a node that `allowed` accepts. Lists, parentheses and runs are searched through.
pub(crate) fn non_literal<'a>(nodes: &'a [Node], allowed: &impl Fn(&NodeKind) -> bool) -> Option<&'a Node> {
    nodes.iter().find_map(|node| match &node.kind {
        NodeKind::Literal(_) | NodeKind::Number { .. } | NodeKind::Zilde(_) | NodeKind::Function(Builtin::Primitive(Primitive::Keys(false))) => None,
        NodeKind::ArrayLiteral { cells, .. } => cells.iter().find_map(|cell| non_literal(cell, allowed)),
        NodeKind::Group(nodes) | NodeKind::Run(nodes) => non_literal(nodes, allowed), kind if allowed(kind) => None, _ => Some(node)
    })
}

/// Items separated by spaces: each run of nodes with no space between them is one item.
fn items(nodes: Vec<Node>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let mut runs: Vec<Vec<Node>> = Vec::new();
    for node in nodes {
        match runs.last_mut() { Some(run) if touching(run.last().unwrap(), &node) => run.push(node), _ => runs.push(vec![node]) }
    }
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

/// Literals separated by spaces form a strand, which becomes one vector literal. A literal that touches the one before it starts a new strand: in `⍣3'b'`, `3` is the operand and `'b'` the argument. A number and the digit powers that touch it group first.
fn strands(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let (mut out, mut strand) = (Vec::with_capacity(nodes.len()), Vec::new());
    let mut nodes = nodes.into_iter().peekable();
    while let Some(node) = nodes.next() {
        if matches!(node.kind, NodeKind::Literal(_) | NodeKind::Number { .. } | NodeKind::Zilde(_)) {
            // A digit power binds to the one number before it, before literals strand, so `2 3²` is `2 9`: the number and its powers group, which the binder folds (`tree::Postfix`).
            let mut powered = vec![node];
            while matches!(powered[0].kind, NodeKind::Number { .. } | NodeKind::Literal(Value::Number(_))) && let Some(next) = nodes.peek()
                && matches!(next.kind, NodeKind::BoundOperator(OperatorKind::Power, _) | NodeKind::Operator(OperatorKind::Inverse))
                && touching(powered.last().expect("the number"), next) { powered.push(nodes.next().expect("the power peeked")); }
            if powered.len() > 1 {
                close_strand(&mut strand, &mut out)?;
                out.push(Node { span: cover(&powered), kind: NodeKind::Group(powered) });
                continue;
            }
            let node = powered.pop().expect("the literal");
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
    let cells = strand.drain(..).map(|node| vec![node]).collect();
    out.push(Node { kind: NodeKind::ArrayLiteral { cells, form: ListForm::Items, record: false }, span });
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
    match &node.kind {
        NodeKind::Function(_) | NodeKind::Operator(_) | NodeKind::BoundOperator(..) | NodeKind::Dfn(_) => true,
        NodeKind::Group(inner) => inner.iter().all(written_function), _ => false
    }
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

/// Close the last run (`unwrapped`).
fn finish((mut out, mut run): (Vec<Node>, Vec<Node>)) -> Vec<Node> { flush(&mut run, &mut out); unwrapped(out) }

/// `nodes` without the wrapper of a run that is the whole expression, which needs none: the parser's statements and the binder's joined runs (`binder::Compiler::joined`).
pub(crate) fn unwrapped(mut nodes: Vec<Node>) -> Vec<Node> {
    if let [Node { kind: NodeKind::Run(inner), .. }] = nodes.as_mut_slice() { return std::mem::take(inner); }
    nodes
}

/// The position of the `:` in a bracketed `key:value` item: its first `:`, which follows keys, with no function before them.
pub(crate) fn key_colon(nodes: &[Node]) -> Option<usize> {
    let i = nodes.iter().position(|n| n.kind.primitive() == Some(Primitive::Keys(false)))?;
    (i > 0 && nodes[..i].iter().all(key_node)).then_some(i)
}

/// Whether a node can stand before `:` as a key: a literal, a name, a group or brackets.
pub(crate) fn key_node(n: &Node) -> bool {
    matches!(
        n.kind, NodeKind::Literal(_) | NodeKind::Number { .. } | NodeKind::Zilde(_) | NodeKind::Name(..) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. }
    )
}

/// The program in `source`, with numbers that have no marker read as floats of the current settings' default width. The current settings also give literals their kinds. They're the session's within an evaluation or `Session::parse`, and the defaults elsewhere, as in a check of structure without evaluation. A complete input can still have a binding or domain error.
pub fn parse(source: Arc<Source>) -> ParseStatus { parse_with_aliases(source, false) }

pub(crate) fn parse_with_aliases(source: Arc<Source>, aliases: bool) -> ParseStatus {
    let tokens = match lex(&source, aliases, None) { Ok(tokens) => tokens, Err(failure) => return failure.into() };
    match (Parser { tokens: &tokens, pos: 0 }).pieces(None).and_then(statements) {
        Ok(statements) => {
            ParseStatus::Complete(Parsed { statements: statements.into_iter().map(|nodes| Statement { nodes, kind: StatementKind::Expression }).collect() })
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

/// The parse tree of `text`. Kinds are `NodeKind` variant names, plus `Program`, `Statement`, `Body` for each dfn body, `Cell` for each array-literal cell, `Strand` for each strand of literals, `Comment`, and `Error` for lines that don't parse. A `Strand` has a `Literal` child for each item. Separators have no nodes.
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
                if let Some(span) = &mut error.span { *span = Span { source: source.clone(), range: span.range.start + start..span.range.end + start }; }
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
    if let Some(strand) = strand_node(node) { return strand; }
    let children = match &node.kind {
        NodeKind::Group(nodes) | NodeKind::Run(nodes) => nodes.iter().map(syntax_node).collect(),
        NodeKind::Scope(target, body) => vec![syntax_node(target), syntax_node(body)],
        NodeKind::ArrayLiteral { cells, .. } => cells.iter().map(|c| list_node("Cell", c)).collect(),
        NodeKind::Dfn(definition) => definition.bodies.iter().map(|body| body_node(body, &definition.span)).collect(), _ => Vec::new()
    };
    SyntaxNode::new(node.kind.name(), node.span.range.clone(), children)
}

/// Literal strands retain their item syntax and spans.
fn strand_node(node: &Node) -> Option<SyntaxNode> {
    let NodeKind::ArrayLiteral { cells, form: ListForm::Items, record: false } = &node.kind else { return None; };
    if node.span.text().starts_with('[') { return None; }
    Some(SyntaxNode::new("Strand", node.span.range.clone(), cells.iter().flat_map(|cell| cell.iter().map(syntax_node)).collect()))
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
            Self::Literal(_) | Self::Number { .. } | Self::Zilde(_) => "Literal", Self::Function(_) => "Function",
            Self::Operator(_) | Self::BoundOperator(..) => "Operator", Self::Name(..) => "Name", Self::System(_) => "System", Self::Assign => "Assign",
            Self::Output => "Output", Self::ErrorGuard => "ErrorGuard", Self::Group(_) => "Group", Self::Run(_) => "Run", Self::Subscript(_) => "Subscript",
            Self::Scope(..) => "Scope", Self::ArrayLiteral { .. } => "ArrayLiteral", Self::Dfn(_) => "Dfn", Self::Joiner(_) => "Joiner",
            Self::Letter(_) => "Letter"
        }
    }
    /// The primitive that a glyph's node holds, or `None` for any other node.
    pub(crate) fn primitive(&self) -> Option<Primitive> { match self { Self::Function(Builtin::Primitive(p)) => Some(*p), _ => None } }
}
