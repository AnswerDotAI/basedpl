use crate::{
    number::Math,
    primitive::{OperatorKind, Primitive, Superscript},
    Error, ErrorAt, ErrorKind, Number, Source, Span, Value,
};
use std::{iter::Peekable, str::CharIndices, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Literal(Value),
    Function(Primitive),
    Operator(OperatorKind),
    Name(String),
    System(String),
    Assign,
    Pipe,
    Pipeline(Vec<Vec<Node>>),
    Output,
    ErrorGuard,
    Group(Vec<Node>),
    /// A run of nodes with no spaces between them, evaluated before its neighbours.
    Run(Vec<Node>),
    /// A subscript, as in `v₁`: `(1⌷v)` for the item just before it, which binding finds after dot access.
    Subscript(i64),
    /// Items of a bracketed list, or with `block` the major cells of an array. `record`: at least one item is `key:value`, and the items build one keyed vector.
    ArrayLiteral { cells: Vec<Vec<Node>>, block: bool, record: bool },
    Dfn(Arc<Definition>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefinitionKind { Function, MonadicOperator, DyadicOperator }

#[derive(Debug)]
pub(crate) struct Definition { pub bodies: Vec<Vec<Statement>>, pub span: Span, pub kind: DefinitionKind }

fn definition_kind(nodes: &[Node]) -> DefinitionKind {
    nodes
        .iter()
        .map(|n| match &n.kind {
            NodeKind::Name(name) if name == "⍹" => DefinitionKind::DyadicOperator,
            NodeKind::Name(name) if name == "⍶" => DefinitionKind::MonadicOperator,
            NodeKind::Group(nodes) | NodeKind::Run(nodes) => definition_kind(nodes),
            NodeKind::ArrayLiteral { cells, .. } => cells.iter().map(|nodes| definition_kind(nodes)).max().unwrap_or(DefinitionKind::Function),
            NodeKind::Pipeline(stages) => stages.iter().map(|nodes| definition_kind(nodes)).max().unwrap_or(DefinitionKind::Function),
            _ => DefinitionKind::Function, // Nested definitions classify their own bodies.
        })
        .max()
        .unwrap_or(DefinitionKind::Function)
}

#[derive(Clone, Debug)]
pub(crate) struct Node { pub kind: NodeKind, pub span: Span }

/// Structural syntax, not an arithmetic AST. Name roles will be resolved during binding.
#[derive(Clone, Debug)]
pub struct Parsed { pub(crate) statements: Vec<Statement> }

#[derive(Clone, Debug)]
pub(crate) struct Statement { pub nodes: Vec<Node>, pub kind: StatementKind }

#[derive(Clone, Copy, Debug)]
/// A predicate ends with `?`. When it fails, the dfn moves on to its next body.
pub(crate) enum StatementKind {
    Expression,
    DefaultArgument,
    Predicate,
    ErrorGuard { index: usize },
}

fn statement(nodes: Vec<Node>) -> Result<Statement, ParseFailure> {
    let mut guard = None;
    for (i, node) in nodes.iter().enumerate() {
        if matches!(node.kind, NodeKind::ErrorGuard) {
            if i == 0 { return Err(ParseFailure::Invalid(node.span.error(ErrorKind::Syntax, "error guard needs error numbers"))); }
            if guard.is_some() { return Err(ParseFailure::Invalid(node.span.error(ErrorKind::Syntax, "a statement can contain only one error guard"))); }
            guard = Some(i);
        }
    }
    let kind = if let Some(index) = guard {
        StatementKind::ErrorGuard { index }
    } else if matches!(&nodes[0].kind, NodeKind::Name(name) if name == "⍺") && matches!(nodes.get(1).map(|n| &n.kind), Some(NodeKind::Assign)) {
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
    Pipe,
    BraceOpen,
    BraceClose,
    Literal(Value),
    Function(Primitive),
    Operator(OperatorKind),
    /// A subscript integer, such as `₁` or `₋₁`, which selects a major cell of the item just before it.
    Subscript(i64),
    Open,
    /// A closing parenthesis, and whether `ₓ` follows it.
    Close(bool),
    BracketOpen,
    /// A closing bracket, and whether `ₓ` follows it.
    BracketClose(bool),
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

fn digits(chars: &mut Peekable<CharIndices<'_>>) -> usize {
    let mut count = 0;
    while chars.peek().is_some_and(|(_, c)| c.is_ascii_digit()) {
        chars.next();
        count += 1;
    }
    count
}

fn real_literal(chars: &mut Peekable<CharIndices<'_>>) -> Result<(), &'static str> {
    if chars.peek().is_some_and(|(_, c)| *c == '¯') { chars.next(); }
    if chars.peek().is_some_and(|(_, c)| *c == '∞') {
        chars.next();
        return Ok(());
    }
    let mut count = digits(chars);
    if chars.peek().is_some_and(|(_, c)| *c == '.') && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit()) {
        chars.next();
        count += digits(chars);
    }
    if count == 0 { return Err("expected digits in numeric literal"); }
    if suffix(chars, EXPONENT) {
        chars.next_if(|&(_, c)| c == '¯');
        if digits(chars) == 0 { return Err("expected exponent digits (use ¯ for a negative exponent)"); }
    }
    Ok(())
}

/// A numeric suffix: the plain letters that spell it, and its subscript.
type Suffix = (&'static [char], char);
pub(crate) const EXPONENT: Suffix = (&['e', 'E'], 'ₑ');
const IMAGINARY: Suffix = (&['J', 'j'], 'ⱼ');
const DENOMINATOR: Suffix = (&['r'], 'ᵣ');
const EXACT: Suffix = (&[], 'ₓ');
const SUFFIXES: [Suffix; 4] = [EXPONENT, IMAGINARY, DENOMINATOR, EXACT];

/// Whether `c` is the subscript of a numeric suffix.
pub(crate) fn subscript(c: char) -> bool { SUFFIXES.iter().any(|s| s.1 == c) }

/// Whether `chars` starts a part of a number: a digit, `¯`, `∞`, or a point before a digit.
fn number_part(mut chars: impl Iterator<Item = (usize, char)>) -> bool {
    match chars.next() {
        Some((_, c)) if c.is_ascii_digit() || matches!(c, '¯' | '∞') => true,
        Some((_, '.')) => chars.next().is_some_and(|(_, c)| c.is_ascii_digit()),
        _ => false,
    }
}

/// Whether `suffix` comes next: its subscript, or one of its plain letters with a number part after it, as in `1e5`. A plain
/// letter before anything else starts a name, so `2edges` is `2` beside `edges`.
fn suffix_follows(chars: &Peekable<CharIndices<'_>>, (plain, sub): Suffix) -> bool {
    let mut ahead = chars.clone();
    match ahead.next() { Some((_, c)) if c == sub => true, Some((_, c)) if plain.contains(&c) => number_part(ahead), _ => false }
}

/// Consumes `suffix` when it comes next, and gives whether it did.
fn suffix(chars: &mut Peekable<CharIndices<'_>>, suffix: Suffix) -> bool {
    let follows = suffix_follows(chars, suffix);
    if follows { chars.next(); }
    follows
}

/// The number that literal text writes, read by `Number::parse` after each subscript suffix becomes its plain letter. `•vfi`
/// shares `Number::parse` and reads only the plain letters.
fn literal_number(text: &str) -> Result<Number, ErrorKind> {
    Number::parse(
        &text
            .chars()
            .map(|c| SUFFIXES.iter().find(|s| s.1 == c).and_then(|s| s.0.first()).copied().unwrap_or(c))
            .collect::<String>(),
    )
}

/// A number written inside `[…]ₓ`. An integer's digits read exactly, and a whole float becomes exact.
fn exact_number(text: &str) -> Result<Number, ErrorKind> {
    if text.chars().all(|c| c.is_ascii_digit() || c == '¯') { return Number::parse(&format!("{text}ₓ")); }
    literal_number(text)?.marked_exact().ok_or(ErrorKind::Domain)
}

/// The empty numeric vector: `⍬ₓ` when `exact`, and `⍬` otherwise.
pub(crate) fn zilde(exact: bool) -> Value {
    let fill = if exact { Number::from_integer(0) } else { Number::from(0.0) };
    Value::empty(vec![0], Value::Number(fill)).unwrap()
}

/// Reads the numbers and `⍬`s inside each `[…]ₓ` and `(…)ₓ` again, as exact, from their own source text, at any depth.
fn mark_exact(tokens: &mut [Token]) -> Result<(), Error> {
    let mut open = Vec::new();
    for i in 0..tokens.len() {
        match tokens[i].kind {
            TokenKind::Open | TokenKind::BracketOpen => open.push(i),
            TokenKind::Close(marked) | TokenKind::BracketClose(marked) => {
                let Some(start) = open.pop() else { continue };
                if !marked { continue; }
                for token in &mut tokens[start + 1..i] {
                    let text = &token.span.source.text[token.span.range.clone()];
                    let value = match &token.kind {
                        TokenKind::Literal(Value::Number(_)) if !text.starts_with('$') => {
                            Value::Number(exact_number(text).error_at(&token.span, "ₓ marks whole numbers exact")?)
                        }
                        TokenKind::Literal(_) if text == "⍬" => zilde(true),
                        _ => continue,
                    };
                    token.kind = TokenKind::Literal(value);
                }
            }
            _ => (),
        }
    }
    Ok(())
}

/// The offset of the next character, or the end of the text.
fn position(chars: &mut Peekable<CharIndices<'_>>, len: usize) -> usize { chars.peek().map_or(len, |&(i, _)| i) }

/// The end of a character that would run on from a numeric literal: a point, a numeric suffix, or a digit when `digit` is set.
fn run_on(chars: &mut Peekable<CharIndices<'_>>, digit: bool) -> Option<usize> {
    let &(i, c) = chars.peek()?;
    let suffix = SUFFIXES.into_iter().any(|s| suffix_follows(chars, s));
    ((digit && c.is_ascii_digit()) || c == '.' || suffix).then(|| i + c.len_utf8())
}

/// Whether `c` can start or continue a name: a letter that isn't a glyph, a superscript or a numeric suffix, or `_`, `∆` or
/// `⍙`. Digits end a name.
pub(crate) fn name_char(c: char) -> bool {
    ((c.is_alphabetic() && !(matches!(c, 'ᵀ' | 'ᵘ') || subscript(c))) || matches!(c, '_' | '∆' | '⍙')) && Primitive::from_glyph(c).is_none()
}

const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
const SUBSCRIPT_DIGITS: &str = "₀₁₂₃₄₅₆₇₈₉";
/// The characters that write a superscript, for the editors' glyph lists.
#[cfg(feature = "python")]
pub(crate) fn superscripts() -> String { [SUPERSCRIPT_DIGITS, "⁻ᵀᵘ"].concat() }
/// The characters that write a subscript, for the editors' glyph lists.
#[cfg(feature = "python")]
pub(crate) fn subscripts() -> String { [SUBSCRIPT_DIGITS, "₋"].concat() }

/// The value of `c` among `digits`, such as `²` among the superscript digits.
fn script_digit(digits: &str, c: char) -> Option<i64> { digits.chars().position(|d| d == c).map(|i| i as i64) }

/// The integer that a superscript or subscript writes, from its first character `c`: a digit, or the minus (`⁻` or `₋`) of a
/// negative integer. `what` names the kind in errors.
fn script_integer(c: char, chars: &mut Peekable<CharIndices<'_>>, digits: &str, what: &str) -> Result<i64, (ErrorKind, String)> {
    let (mut value, mut count) = script_digit(digits, c).map_or((0, 0), |d| (d, 1));
    while let Some(d) = chars.peek().and_then(|&(_, c)| script_digit(digits, c)) {
        chars.next();
        count += 1;
        value = value.checked_mul(10).and_then(|v| v.checked_add(d)).ok_or((ErrorKind::Limit, format!("{what} is too large")))?;
    }
    if count == 0 { return Err((ErrorKind::Syntax, format!("{c} needs a {what} digit after it"))); }
    Ok(if script_digit(digits, c).is_some() { value } else { -value })
}

const UNQUOTED: &str = "_ quotes the next character into a name, and there is none";

/// Reads the rest of a name. A letter, `∆` or `⍙` continues it, and a digit ends it, as it ends a glyph. `_` quotes the next
/// character into the name, and a quoted digit takes the digits after it, so `x_12` is one name. After a dot, digits continue a
/// key, so `T.q1` names the key `q1`. A lone `_`, before a space, `]`, `)`, `←`, `;`, `⋄` or the end, is the name `_`. Any other
/// `_` with nothing to quote gives the position after it.
fn name(chars: &mut Peekable<CharIndices<'_>>, key: bool) -> Result<(), usize> {
    let lone = |c: Option<&(usize, char)>| c.is_none_or(|&(_, c)| c.is_whitespace() || matches!(c, ']' | ')' | '←' | ';' | '⋄'));
    let mut first = true;
    loop {
        match chars.peek() {
            Some(&(_, '_')) => {
                let (i, _) = chars.next().unwrap();
                if first && lone(chars.peek()) { return Ok(()); }
                match chars.next() {
                    Some((_, q)) if !q.is_whitespace() => {
                        if q.is_ascii_digit() { digits(chars); }
                    }
                    _ => return Err(i + 1),
                }
            }
            Some(&(_, c)) if name_char(c) || (key && c.is_ascii_digit()) => {
                chars.next();
            }
            _ => return Ok(()),
        }
        first = false;
    }
}

/// Whether a line break beside a token of `kind` separates nothing. `before` says the token comes before the line break.
/// Every other line break reads as `⋄`.
fn absorbs_line_break(kind: &TokenKind, before: bool) -> bool {
    use TokenKind::*;
    match kind {
        Separator | Semicolon | Predicate => true,
        Open | BracketOpen | BraceOpen => before,
        Close(_) | BracketClose(_) | BraceClose => !before,
        _ => false,
    }
}

fn lex(source: &Arc<Source>) -> Result<Vec<Token>, Error> {
    let len = source.text.len();
    let mut chars = source.text.char_indices().peekable();
    // A script's first line can name its interpreter.
    if source.text.starts_with("#!") { while chars.next_if(|&(_, c)| c != '\n').is_some() {} }
    let (mut tokens, mut line_break) = (Vec::new(), None);
    while let Some(&(start, c)) = chars.peek() {
        let span = |end| Span { source: source.clone(), range: start..end };
        let kind = if c.is_ascii_digit() || matches!(c, '¯' | '∞') || (c == '.' && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit())) {
            real_literal(&mut chars).map_err(|message| span(position(&mut chars, len)).error(ErrorKind::Syntax, message))?;
            if suffix(&mut chars, IMAGINARY) {
                real_literal(&mut chars).map_err(|message| span(position(&mut chars, len)).error(ErrorKind::Syntax, message))?;
                if let Some(end) = run_on(&mut chars, false) { return Err(span(end).error(ErrorKind::Syntax, "invalid complex numeric literal")); }
            } else {
                let rational = suffix(&mut chars, DENOMINATOR);
                if rational {
                    chars.next_if(|&(_, c)| c == '¯');
                    if digits(&mut chars) == 0 { return Err(span(position(&mut chars, len)).error(ErrorKind::Syntax, "expected integer denominator")); }
                }
                if rational || suffix(&mut chars, EXACT) {
                    if let Some(end) = run_on(&mut chars, true) { return Err(span(end).error(ErrorKind::Syntax, "invalid exact numeric literal")); }
                }
            }
            let end = position(&mut chars, len);
            let n = literal_number(&source.text[start..end])
                .error_at(&span(end), "invalid numeric literal (real values, finite complex components or integer components with ₓ and r required)")?;
            TokenKind::Literal(Value::number(n).unwrap())
        } else if name_char(c) {
            name(&mut chars, source.text[..start].ends_with('.')).map_err(|end| span(end).error(ErrorKind::Syntax, UNQUOTED))?;
            TokenKind::Name(source.text[start..position(&mut chars, len)].to_owned())
        } else {
            chars.next();
            match c {
                '\'' => {
                    let unclosed = |at| Err(span(at).error(ErrorKind::Syntax, "unclosed character literal"));
                    let c = match chars.next() { Some((_, c)) if c != '\n' => c, next => return unclosed(next.map_or(len, |(i, _)| i)) };
                    match chars.peek() {
                        Some((_, '\'')) => {
                            chars.next();
                        }
                        None | Some((_, '\n')) => return unclosed(position(&mut chars, len)),
                        Some(&(i, c)) => {
                            return Err(span(i + c.len_utf8()).error(ErrorKind::Syntax, "a character literal holds one character (strings use double quotes)"))
                        }
                    }
                    TokenKind::Literal(Value::Character(c))
                }
                '"' => {
                    let mut text = String::new();
                    loop {
                        match chars.next() {
                            Some((_, '"')) if chars.next_if(|&(_, c)| c == '"').is_some() => text.push('"'),
                            Some((_, '"')) => break,
                            Some((_, '\n')) | None => return Err(span(position(&mut chars, len)).error(ErrorKind::Syntax, "unclosed string")),
                            Some((_, c)) => text.push(c),
                        }
                    }
                    TokenKind::Literal(crate::keyed::text(&text))
                }
                '⍬' => TokenKind::Literal(zilde(suffix(&mut chars, EXACT))),
                // A literal constant: `$` and one letter.
                '$' => {
                    let value = match chars.next() {
                        Some((_, 't')) => Number::from_bool(true),
                        Some((_, 'f')) => Number::from_bool(false),
                        Some((_, 'n')) => Number::from(f64::NAN),
                        _ => return Err(span(position(&mut chars, len)).error(ErrorKind::Syntax, "$ needs t, f or n")),
                    };
                    if chars.peek().is_some_and(|&(_, c)| name_char(c) || c.is_ascii_digit()) {
                        return Err(span(position(&mut chars, len)).error(ErrorKind::Syntax, "a constant is $ and one letter: $t, $f or $n"));
                    }
                    TokenKind::Literal(Value::Number(value))
                }
                '⍛' => return Err(span(start + c.len_utf8()).error(ErrorKind::Syntax, "⍛ is retired: use ↣ or ↢ to bind or preprocess, and ∘ for Atop")),
                '(' => TokenKind::Open,
                ')' => TokenKind::Close(suffix(&mut chars, EXACT)),
                '[' => TokenKind::BracketOpen,
                ']' => TokenKind::BracketClose(suffix(&mut chars, EXACT)),
                ';' => TokenKind::Semicolon,
                '\n' => {
                    if tokens.last().is_some_and(|t: &Token| !absorbs_line_break(&t.kind, true)) { line_break = Some(span(start + 1)); }
                    continue;
                }
                '⋄' => TokenKind::Separator,
                '←' => TokenKind::Assign,
                '→' => TokenKind::Pipe,
                '•' if chars.peek().is_some_and(|&(_, c)| name_char(c)) => {
                    name(&mut chars, false).map_err(|end| span(end).error(ErrorKind::Syntax, UNQUOTED))?;
                    TokenKind::System(source.text[start..position(&mut chars, len)].to_owned())
                }
                '⎕' => TokenKind::Output,
                ':' if chars.peek().is_some_and(|(_, c)| *c == ':') => {
                    chars.next();
                    TokenKind::ErrorGuard
                }
                ':' => TokenKind::Function(Primitive::Keys),
                '?' => TokenKind::Predicate,
                '{' => TokenKind::BraceOpen,
                '}' => TokenKind::BraceClose,
                '⍺' | '⍵' | '⍶' | '⍹' | '∇' | '⍢' => TokenKind::Name(c.to_string()),
                '⍝' => {
                    while chars.peek().is_some_and(|(_, c)| *c != '\n') { chars.next(); }
                    continue;
                }
                'ᵀ' => TokenKind::Operator(OperatorKind::Super(Superscript::Transpose)),
                'ᵘ' => TokenKind::Operator(OperatorKind::Super(Superscript::Unit)),
                c if c == '⁻' || script_digit(SUPERSCRIPT_DIGITS, c).is_some() => {
                    let power =
                        script_integer(c, &mut chars, SUPERSCRIPT_DIGITS, "superscript").map_err(|(k, m)| span(position(&mut chars, len)).error(k, m))?;
                    TokenKind::Operator(OperatorKind::Super(Superscript::Power(power)))
                }
                c if c == '₋' || script_digit(SUBSCRIPT_DIGITS, c).is_some() => TokenKind::Subscript(
                    script_integer(c, &mut chars, SUBSCRIPT_DIGITS, "subscript").map_err(|(k, m)| span(position(&mut chars, len)).error(k, m))?,
                ),
                c if c.is_whitespace() => continue,
                c => match (OperatorKind::from_glyph(c), Primitive::from_glyph(c)) {
                    (Some(op), _) => TokenKind::Operator(op),
                    (_, Some(f)) => TokenKind::Function(f),
                    _ => return Err(span(start + c.len_utf8()).error(ErrorKind::Unsupported, format!("{c:?} is not supported yet"))),
                },
            }
        };
        let end = position(&mut chars, len);
        if let Some(at) = line_break.take().filter(|_| !absorbs_line_break(&kind, false)) { tokens.push(Token { kind: TokenKind::Separator, span: at }); }
        tokens.push(Token { kind, span: span(end) });
    }
    mark_exact(&mut tokens)?;
    Ok(tokens)
}

enum ParseFailure { Incomplete(Error), Invalid(Error) }

fn invalid(span: &Span, message: &str) -> ParseFailure { ParseFailure::Invalid(span.error(ErrorKind::Syntax, message)) }

pub(crate) fn cover(nodes: &[Node]) -> Span {
    Span { source: nodes[0].span.source.clone(), range: nodes[0].span.range.start..nodes[nodes.len() - 1].span.range.end }
}

/// Nodes and separators, before the enclosing delimiters give the separators their meaning.
enum Piece {
    Node(Node),
    Diamond,
    Semicolon(Span),
    Predicate(Span),
}

struct Parser<'a> { tokens: &'a [Token], pos: usize }
impl Parser<'_> {
    fn pieces(&mut self, open: Option<&Token>) -> Result<Vec<Piece>, ParseFailure> {
        let mut pieces = Vec::new();
        while let Some(token) = self.tokens.get(self.pos) {
            self.pos += 1;
            let mut span = token.span.clone();
            let kind = match &token.kind {
                TokenKind::Separator => {
                    pieces.push(Piece::Diamond);
                    continue;
                }
                TokenKind::Semicolon => {
                    pieces.push(Piece::Semicolon(span));
                    continue;
                }
                TokenKind::Predicate => {
                    pieces.push(Piece::Predicate(span));
                    continue;
                }
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
                    let marked = matches!(self.tokens[self.pos - 1].kind, TokenKind::Close(true) | TokenKind::BracketClose(true));
                    match token.kind {
                        TokenKind::BraceOpen => {
                            let bodies = bodies(inner, &span)?;
                            let kind = bodies.iter().flatten().map(|s| definition_kind(&s.nodes)).max().unwrap_or(DefinitionKind::Function);
                            NodeKind::Dfn(Arc::new(Definition { bodies, span: span.clone(), kind }))
                        }
                        TokenKind::Open if marked => exact_literal(parenthesised(inner, &span)?, &span)?,
                        TokenKind::Open => parenthesised(inner, &span)?,
                        _ if marked => exact_literal(brackets(inner, &span)?, &span)?,
                        _ => brackets(inner, &span)?,
                    }
                }
                TokenKind::Literal(a) => NodeKind::Literal(a.clone()),
                TokenKind::Function(f) => NodeKind::Function(*f),
                TokenKind::Operator(op) => NodeKind::Operator(*op),
                TokenKind::Subscript(index) => NodeKind::Subscript(*index),
                TokenKind::Name(name) => NodeKind::Name(name.clone()),
                TokenKind::System(name) => NodeKind::System(name.clone()),
                TokenKind::Assign => NodeKind::Assign,
                TokenKind::Pipe => NodeKind::Pipe,
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

/// The nodes between separators. `⋄` always separates, and `;` only when `semicolons` is set. Elsewhere `;` is an error.
/// `?` belongs only in dfns.
fn split(pieces: Vec<Piece>, semicolons: bool) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let mut parts = vec![Vec::new()];
    for piece in pieces {
        match piece {
            Piece::Node(n) => parts.last_mut().unwrap().push(n),
            Piece::Semicolon(s) if !semicolons => return Err(invalid(&s, "; separates items in brackets and bodies in braces, and nowhere else")),
            Piece::Predicate(s) => return Err(invalid(&s, "? ends a predicate, which belongs in a dfn")),
            _ => parts.push(Vec::new()),
        }
    }
    Ok(parts)
}

/// Statements at the top level: a line break or `⋄` ends one.
fn statements(pieces: Vec<Piece>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    split(pieces, false)?.into_iter().filter(|nodes| !nodes.is_empty()).map(expression).collect()
}

/// A dfn's bodies, separated by `;`. A line break or `⋄` ends a statement, and `?` ends a predicate. Every body but the
/// last needs a predicate: a body without one always returns, so the bodies after it could never run. A body that ends
/// after its predicate returns no result.
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

/// Parts separated by `⋄`, where a trailing `⋄` ends the last row: `(1 2 ⋄)` is `[[1 2]]`, and `[1 2 ⋄]` is a 1×2 matrix.
fn diamond_rows(mut parts: Vec<Vec<Node>>, span: &Span) -> Result<Vec<Vec<Node>>, ParseFailure> {
    if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) { parts.pop(); }
    if parts.iter().any(Vec::is_empty) { return Err(invalid(span, "empty row between diamonds")); }
    Ok(parts)
}

/// One row read as a bracketed list of its items.
fn row(nodes: Vec<Node>) -> Result<Node, ParseFailure> {
    let span = cover(&nodes);
    let cells = items(nodes)?;
    Ok(Node { kind: NodeKind::ArrayLiteral { record: cells.iter().any(|c| key_colon(c).is_some()), cells, block: false }, span })
}

/// `(4 ⋄ 4 5)` is `[[4] [4 5]]`. Its items are the rows, each read as a bracketed list.
fn rows(pieces: Vec<Piece>, span: &Span) -> Result<NodeKind, ParseFailure> {
    let cells = diamond_rows(split(pieces, false)?, span)?.into_iter().map(|nodes| Ok(vec![row(nodes)?])).collect::<Result<_, _>>()?;
    Ok(NodeKind::ArrayLiteral { cells, block: false, record: false })
}

/// Whether nothing comes between two nodes in the source.
pub(crate) fn touching(a: &Node, b: &Node) -> bool { a.span.range.end == b.span.range.start }

/// Brackets build arrays. A space separates items, `;` separates items that contain spaces, and `⋄` or a line break
/// separates major cells. Brackets round one item make a one-item vector.
fn brackets(pieces: Vec<Piece>, span: &Span) -> Result<NodeKind, ParseFailure> {
    let semicolon = pieces.iter().any(|p| matches!(p, Piece::Semicolon(_)));
    let diamond = pieces.iter().any(|p| matches!(p, Piece::Diamond));
    if semicolon && diamond { return Err(invalid(span, "; and ⋄ cannot both separate items in one pair of brackets")); }
    let mut parts = split(pieces, true)?;
    if diamond {
        // A row that is a single item is that item's major cell.
        let cells = diamond_rows(parts, span)?
            .into_iter()
            .map(|nodes| {
                let row = row(nodes)?;
                Ok(match row.kind { NodeKind::ArrayLiteral { mut cells, .. } if cells.len() == 1 => cells.pop().unwrap(), _ => vec![row] })
            })
            .collect::<Result<_, _>>()?;
        return Ok(NodeKind::ArrayLiteral { cells, block: true, record: false });
    }
    let cells = if semicolon {
        if parts.last().is_some_and(Vec::is_empty) { return Err(invalid(span, "a trailing ; leaves an empty item: [x] is already a one-item vector")); }
        if parts.iter().any(Vec::is_empty) { return Err(invalid(span, "empty item between semicolons")); }
        parts.into_iter().map(expression).collect::<Result<Vec<_>, _>>()?
    } else { items(parts.pop().unwrap())? };
    Ok(NodeKind::ArrayLiteral { record: cells.iter().any(|c| key_colon(c).is_some()), cells, block: false })
}

/// `[…]ₓ` or `(…)ₓ`, whose numbers the lexer has already read as exact. Every item must be a literal, and `[]ₓ` is `⍬ₓ`.
fn exact_literal(kind: NodeKind, span: &Span) -> Result<NodeKind, ParseFailure> {
    if matches!(&kind, NodeKind::ArrayLiteral { cells, .. } if cells.is_empty()) { return Ok(NodeKind::Literal(zilde(true))); }
    let node = Node { kind, span: span.clone() };
    literal_items(std::slice::from_ref(&node))?;
    Ok(node.kind)
}

/// Checks that every item is a literal, at any depth.
fn literal_items(nodes: &[Node]) -> Result<(), ParseFailure> {
    for node in nodes {
        match &node.kind {
            NodeKind::Literal(_) | NodeKind::Function(Primitive::Keys) => (),
            NodeKind::ArrayLiteral { cells, .. } => {
                for cell in cells { literal_items(cell)?; }
            }
            NodeKind::Group(nodes) | NodeKind::Run(nodes) => literal_items(nodes)?,
            _ => return Err(invalid(&node.span, "ₓ needs every item to be a literal")),
        }
    }
    Ok(())
}

/// Items separated by spaces: each run of nodes with no space between them is one item.
fn items(nodes: Vec<Node>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let mut runs: Vec<Vec<Node>> = Vec::new();
    for node in nodes { match runs.last_mut() { Some(run) if touching(run.last().unwrap(), &node) => run.push(node), _ => runs.push(vec![node]) } }
    runs.into_iter().map(expression).collect()
}

/// An expression outside brackets, or one item between semicolons: strands, then runs, then pipelines.
fn expression(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let nodes = pipelines(runs(strands(nodes)?)?)?;
    subscripts_follow_items(&nodes)?;
    Ok(nodes)
}

/// Fails unless each subscript follows a node in its list. That node is the item the subscript touches, or after a space the run before it.
fn subscripts_follow_items(nodes: &[Node]) -> Result<(), ParseFailure> {
    if let Some(n @ Node { kind: NodeKind::Subscript(_), .. }) = nodes.first() {
        return Err(invalid(&n.span, "a subscript selects from the item just before it, or after a space from the run before it"));
    }
    for n in nodes {
        match &n.kind {
            NodeKind::Run(inner) => subscripts_follow_items(inner)?,
            NodeKind::Pipeline(stages) => stages.iter().try_for_each(|s| subscripts_follow_items(s))?,
            _ => (),
        }
    }
    Ok(())
}

/// Literals separated by spaces form a strand, which becomes one vector literal. A literal that touches the one before it starts a new
/// strand: in `⍣3'b'`, `3` is the operand and `'b'` the argument.
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
                    Superscript::Power(k) => n.math_dyad(Math::Power, &Number::from_integer(k)),
                    Superscript::Transpose => Ok(n.clone()),
                    Superscript::Unit => break,
                };
                node.span.range.end = end;
                let raised = raised.map_err(|m| ParseFailure::Invalid(node.span.error(ErrorKind::Domain, m)))?;
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
    if strand.len() < 2 {
        out.append(strand);
        return Ok(());
    }
    let span = cover(strand);
    let items: Vec<_> = strand.drain(..).map(|n| if let NodeKind::Literal(v) = n.kind { v } else { unreachable!() }).collect();
    let value = Value::new(vec![items.len()], items).map_err(|k| ParseFailure::Invalid(span.error(k, "invalid literal list")))?;
    out.push(Node { kind: NodeKind::Literal(value), span });
    Ok(())
}

/// Spaces separate runs, and pipes and error guards separate them too. The run holding an assignment's target takes in
/// the `←` and its value, which runs to the next error guard. The target stays flat within that run, because assignment chooses
/// its own target. A run alone between separators needs no wrapper.
fn runs(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let (mut result, mut part) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if matches!(node.kind, NodeKind::ErrorGuard) {
            result.extend(assignments(std::mem::take(&mut part))?);
            result.push(node);
        } else { part.push(node) }
    }
    result.extend(assignments(part)?);
    Ok(result)
}

/// Assignments bind from the right, so this works back from the last `←`. A chain such as `a←b←0` stays flat.
fn assignments(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let mut segments = vec![Vec::new()];
    for node in nodes {
        let assign = matches!(node.kind, NodeKind::Assign);
        segments.last_mut().unwrap().push(node);
        if assign { segments.push(Vec::new()); }
    }
    let mut value: std::collections::VecDeque<Node> = pipelines(finish(spaced(segments.pop().unwrap())))?.into();
    while let Some(mut segment) = segments.pop() {
        let assign = segment.pop().unwrap();
        let (mut out, mut target) = spaced(segment);
        // A space before `←` leaves the target in the previous run.
        if target.is_empty() && out.last().is_some_and(|n| !matches!(n.kind, NodeKind::Pipe)) {
            target = match out.pop() { Some(Node { kind: NodeKind::Run(inner), .. }) => inner, Some(n) => vec![n], None => unreachable!() };
        }
        value.push_front(assign);
        for node in target.into_iter().rev() { value.push_front(node); }
        if out.is_empty() { continue; }
        let mut nodes = Vec::from(value);
        // Runs before the target in its stage stay separate from it.
        if out.iter().rposition(|n| matches!(n.kind, NodeKind::Pipe)).map_or(0, |i| i + 1) < out.len() {
            let span = cover(&nodes);
            nodes = vec![Node { kind: NodeKind::Run(nodes), span }];
        }
        out.extend(nodes);
        value = pipelines(out)?.into();
    }
    Ok(value.into())
}

/// Nodes with no space between them form runs, and pipes separate them. Returns the finished runs and the last
/// run, which is still open.
fn spaced(nodes: Vec<Node>) -> (Vec<Node>, Vec<Node>) {
    let (mut out, mut run): (Vec<Node>, Vec<Node>) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if matches!(node.kind, NodeKind::Pipe) {
            flush(&mut run, &mut out);
            out.push(node);
        } else {
            if run.last().is_some_and(|last| !touching(last, &node)) { flush(&mut run, &mut out); }
            run.push(node);
        }
    }
    (out, run)
}

fn flush(run: &mut Vec<Node>, out: &mut Vec<Node>) {
    if run.len() < 2 { out.append(run) } else {
        let span = cover(run);
        out.push(Node { kind: NodeKind::Run(std::mem::take(run)), span });
    }
}

/// Close the last run. A run alone between pipes needs no wrapper.
fn finish((mut out, mut run): (Vec<Node>, Vec<Node>)) -> Vec<Node> {
    flush(&mut run, &mut out);
    let (mut result, mut stage) = (Vec::with_capacity(out.len()), Vec::new());
    let close = |stage: &mut Vec<Node>, result: &mut Vec<Node>| {
        if let [Node { kind: NodeKind::Run(inner), .. }] = stage.as_mut_slice() {
            result.append(inner);
            stage.clear();
        } else { result.append(stage) }
    };
    for node in out {
        if matches!(node.kind, NodeKind::Pipe) {
            close(&mut stage, &mut result);
            result.push(node);
        } else { stage.push(node) }
    }
    close(&mut stage, &mut result);
    result
}

// Assignment encloses the pipeline; error guards separate independent expressions.
fn pipelines(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    if !nodes.iter().any(|n| matches!(n.kind, NodeKind::Pipe)) { return Ok(nodes); }
    let mut result = Vec::new();
    for part in nodes.split_inclusive(|n| matches!(n.kind, NodeKind::ErrorGuard)) {
        let (expression, guard) =
            if part.last().is_some_and(|n| matches!(n.kind, NodeKind::ErrorGuard)) { (&part[..part.len() - 1], part.last()) } else { (part, None) };
        if let Some(pipe) = expression.iter().position(|n| matches!(n.kind, NodeKind::Pipe)) {
            let start = expression[..pipe].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |i| i + 1);
            let stages: Vec<_> = expression[start..].split(|n| matches!(n.kind, NodeKind::Pipe)).map(<[Node]>::to_vec).collect();
            if stages.iter().any(Vec::is_empty) {
                return Err(ParseFailure::Invalid(expression[pipe].span.error(ErrorKind::Syntax, "pipe needs an expression on each side")));
            }
            if stages.iter().skip(1).flatten().any(|n| matches!(n.kind, NodeKind::Assign)) {
                return Err(ParseFailure::Invalid(expression[pipe].span.error(ErrorKind::Syntax, "parenthesize assignment inside a pipe stage")));
            }
            let mut span = expression[start].span.clone();
            span.range.end = expression.last().unwrap().span.range.end;
            result.extend_from_slice(&expression[..start]);
            result.push(Node { kind: NodeKind::Pipeline(stages), span });
        }
        else { result.extend_from_slice(expression); }
        if let Some(guard) = guard { result.push(guard.clone()); }
    }
    Ok(result)
}

/// The position of the `:` in a bracketed `key:value` item: its first `:`, which follows keys, with no function before them.
pub(crate) fn key_colon(nodes: &[Node]) -> Option<usize> {
    let i = nodes.iter().position(|n| matches!(n.kind, NodeKind::Function(Primitive::Keys)))?;
    (i > 0 && nodes[..i].iter().all(key_node)).then_some(i)
}

/// Whether a node can stand before `:` as a key: a literal, a name, a group or brackets.
pub(crate) fn key_node(n: &Node) -> bool { matches!(n.kind, NodeKind::Literal(_) | NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. }) }

/// Check structure without evaluation. A complete input can still have a binding or domain error.
pub fn parse(source: Arc<Source>) -> ParseStatus {
    let tokens = match lex(&source) { Ok(tokens) => tokens, Err(e) => return ParseStatus::Invalid(e) };
    match (Parser { tokens: &tokens, pos: 0 }).pieces(None).and_then(statements) {
        Ok(statements) => {
            ParseStatus::Complete(Parsed { statements: statements.into_iter().map(|nodes| Statement { nodes, kind: StatementKind::Expression }).collect() })
        }
        Err(ParseFailure::Incomplete(e)) => ParseStatus::Incomplete(e),
        Err(ParseFailure::Invalid(e)) => ParseStatus::Invalid(e),
    }
}
