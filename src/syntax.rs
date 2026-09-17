use crate::{primitive::Primitive, Error, ErrorKind, Number, Source, Span};
use std::{iter::Peekable, rc::Rc, str::CharIndices};

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Number(Number),
    Function(Primitive),
    Name(String),
    Assign,
    Output,
    Guard(bool),
    Hybrid,
    Group(Vec<Node>),
    Dfn(Rc<Definition>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DefinitionKind { Function, MonadicOperator, DyadicOperator }

#[derive(Debug)]
pub(crate) struct Definition { pub body: Parsed, pub span: Span, pub kind: DefinitionKind }

fn definition_kind(nodes: &[Node]) -> DefinitionKind {
    nodes
        .iter()
        .map(|n| match &n.kind {
            NodeKind::Name(name) if name == "⍵⍵" => DefinitionKind::DyadicOperator,
            NodeKind::Name(name) if name == "⍺⍺" => DefinitionKind::MonadicOperator,
            NodeKind::Group(nodes) => definition_kind(nodes),
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
pub(crate) struct Statement { pub nodes: Vec<Node>, pub guard: Option<(usize, bool)> }

fn statement(nodes: Vec<Node>) -> Result<Statement, ParseFailure> {
    let mut guard = None;
    for (i, node) in nodes.iter().enumerate() {
        if let NodeKind::Guard(error) = node.kind {
            if i == 0 || i + 1 == nodes.len() { return Err(ParseFailure::Invalid(node.span.error(ErrorKind::Syntax, "guard needs a condition and result"))); }
            if guard.is_some() {
                return Err(ParseFailure::Invalid(node.span.error(ErrorKind::Unsupported, "chained guards on one statement are not implemented yet")));
            }
            guard = Some((i, error));
        }
    }
    Ok(Statement { nodes, guard })
}

#[derive(Debug)]
pub enum ParseStatus { Complete(Parsed), Incomplete(Error), Invalid(Error) }

#[derive(Debug)]
enum TokenKind {
    BraceOpen,
    BraceClose,
    Number(Number),
    Function(Primitive),
    Open,
    Close,
    Newline,
    Separator,
    Name(String),
    Assign,
    Output,
    Guard(bool),
    Hybrid,
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
    let mut count = digits(chars);
    if chars.peek().is_some_and(|(_, c)| *c == '.') {
        chars.next();
        count += digits(chars);
    }
    if count == 0 { return Err("expected digits in numeric literal"); }
    if chars.peek().is_some_and(|(_, c)| matches!(c, 'e' | 'E')) {
        chars.next();
        if chars.peek().is_some_and(|(_, c)| *c == '¯') { chars.next(); }
        if digits(chars) == 0 { return Err("expected exponent digits (use ¯ for a negative exponent)"); }
    }
    Ok(())
}

fn lex(source: &Rc<Source>) -> Result<Vec<Token>, Error> {
    let mut chars = source.text.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some(&(start, c)) = chars.peek() {
        let span = |end| Span { source: source.clone(), range: start..end };
        let kind = if c.is_ascii_digit() || c == '¯' || c == '.' {
            real_literal(&mut chars).map_err(|message| span(chars.peek().map_or(source.text.len(), |(i, _)| *i)).error(ErrorKind::Syntax, message))?;
            if chars.peek().is_some_and(|(_, c)| matches!(c, 'J' | 'j')) {
                chars.next();
                real_literal(&mut chars).map_err(|message| span(chars.peek().map_or(source.text.len(), |(i, _)| *i)).error(ErrorKind::Syntax, message))?;
                if chars.peek().is_some_and(|(_, c)| matches!(c, '.' | 'e' | 'E' | 'x' | 'r' | 'J' | 'j')) {
                    return Err(span(chars.peek().unwrap().0 + 1).error(ErrorKind::Syntax, "invalid complex numeric literal"));
                }
            }
            else if let Some(&(_, suffix @ ('x' | 'r'))) = chars.peek() {
                chars.next();
                if suffix == 'r' {
                    if chars.peek().is_some_and(|(_, c)| *c == '¯') { chars.next(); }
                    if digits(&mut chars) == 0 {
                        let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
                        return Err(span(end).error(ErrorKind::Syntax, "expected integer denominator"));
                    }
                }
                if chars.peek().is_some_and(|(_, c)| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | 'x' | 'r' | 'J' | 'j')) {
                    let end = chars.peek().unwrap().0 + 1;
                    return Err(span(end).error(ErrorKind::Syntax, "invalid exact numeric literal"));
                }
            }
            let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
            let n = Number::parse(&source.text[start..end])
                .map_err(|k| span(end).error(k, "invalid numeric literal (finite real/complex values or integer x/r components required)"))?;
            TokenKind::Number(n)
        } else if c.is_alphabetic() || matches!(c, '_' | '∆' | '⍙') {
            chars.next();
            while chars.peek().is_some_and(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '∆' | '⍙')) { chars.next(); }
            let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
            TokenKind::Name(source.text[start..end].to_owned())
        } else {
            chars.next();
            match c {
                '(' => TokenKind::Open,
                ')' => TokenKind::Close,
                '\n' => TokenKind::Newline,
                '⋄' => TokenKind::Separator,
                '←' => TokenKind::Assign,
                '⎕' => TokenKind::Output,
                ':' => {
                    let error = chars.peek().is_some_and(|(_, c)| *c == ':');
                    if error { chars.next(); }
                    TokenKind::Guard(error)
                }
                '/' => TokenKind::Hybrid,
                '{' => TokenKind::BraceOpen,
                '}' => TokenKind::BraceClose,
                '⍺' | '⍵' | '∇' => {
                    let mut name = c.to_string();
                    if chars.peek().is_some_and(|(_, next)| *next == c) {
                        chars.next();
                        name.push(c);
                    }
                    TokenKind::Name(name)
                }
                '⍝' => {
                    while chars.peek().is_some_and(|(_, c)| *c != '\n') { chars.next(); }
                    continue;
                }
                c if c.is_whitespace() => continue,
                c => match Primitive::from_glyph(c) {
                    Some(f) => TokenKind::Function(f),
                    None => return Err(span(start + c.len_utf8()).error(ErrorKind::Unsupported, format!("{c:?} is not supported yet"))),
                },
            }
        };
        let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
        tokens.push(Token { kind, span: span(end) });
    }
    Ok(tokens)
}

enum ParseFailure { Incomplete(Error), Invalid(Error) }

fn sequence(tokens: &[Token], pos: &mut usize, open: Option<&Span>, depth: usize, in_dfn: bool) -> Result<Vec<Node>, ParseFailure> {
    let mut nodes = Vec::new();
    while let Some(token) = tokens.get(*pos) {
        *pos += 1;
        let kind = match &token.kind {
            TokenKind::Number(n) => NodeKind::Number(n.clone()),
            TokenKind::Function(f) => NodeKind::Function(*f),
            TokenKind::Name(name) => NodeKind::Name(name.clone()),
            TokenKind::Assign => NodeKind::Assign,
            TokenKind::Output => NodeKind::Output,
            TokenKind::Guard(error) => {
                if !in_dfn || open.is_some() { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "guards belong to dfn statements"))); }
                NodeKind::Guard(*error)
            }
            TokenKind::Hybrid => NodeKind::Hybrid,
            TokenKind::BraceOpen => {
                if depth == 128 { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Limit, "definitions nested too deeply"))); }
                let body = dfn_body(tokens, pos, &token.span, depth + 1)?;
                let span = Span { source: token.span.source.clone(), range: token.span.range.start..tokens[*pos - 1].span.range.end };
                let kind = body.statements.iter().map(|s| definition_kind(&s.nodes)).max().unwrap_or(DefinitionKind::Function);
                nodes.push(Node { kind: NodeKind::Dfn(Rc::new(Definition { body, span: span.clone(), kind })), span });
                continue;
            }
            TokenKind::BraceClose if in_dfn && open.is_none() => {
                *pos -= 1;
                return Ok(nodes);
            }
            TokenKind::BraceClose => return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "unexpected closing brace"))),
            TokenKind::Newline if open.is_some() || nodes.is_empty() => continue,
            TokenKind::Newline | TokenKind::Separator if open.is_none() => return Ok(nodes),
            TokenKind::Separator => {
                return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Unsupported, "array-literal separators are not implemented yet")))
            }
            TokenKind::Newline => unreachable!(),
            TokenKind::Open => {
                if depth == 128 { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Limit, "parentheses nested too deeply"))); }
                let group = sequence(tokens, pos, Some(&token.span), depth + 1, in_dfn)?;
                let span = Span { source: token.span.source.clone(), range: token.span.range.start..tokens[*pos - 1].span.range.end };
                nodes.push(Node { kind: NodeKind::Group(group), span });
                continue;
            }
            TokenKind::Close => {
                if open.is_none() { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "unexpected closing parenthesis"))); }
                if nodes.is_empty() {
                    return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Unsupported, "empty parentheses are not an array literal")));
                }
                return Ok(nodes);
            }
        };
        nodes.push(Node { kind, span: token.span.clone() });
    }
    if let Some(span) = open { return Err(ParseFailure::Incomplete(span.error(ErrorKind::Syntax, "unclosed parenthesis"))); }
    Ok(nodes)
}

fn dfn_body(tokens: &[Token], pos: &mut usize, span: &Span, depth: usize) -> Result<Parsed, ParseFailure> {
    let mut statements = Vec::new();
    while let Some(token) = tokens.get(*pos) {
        if matches!(token.kind, TokenKind::BraceClose) {
            *pos += 1;
            return Ok(Parsed { statements });
        }
        let nodes = sequence(tokens, pos, None, depth, true)?;
        if !nodes.is_empty() { statements.push(statement(nodes)?); }
    }
    Err(ParseFailure::Incomplete(span.error(ErrorKind::Syntax, "unclosed definition")))
}

/// Check structure without evaluation. A complete input can still have a binding or domain error.
pub fn parse(source: Rc<Source>) -> ParseStatus {
    let tokens = match lex(&source) { Ok(tokens) => tokens, Err(e) => return ParseStatus::Invalid(e) };
    let mut pos = 0;
    let mut statements = Vec::new();
    while pos < tokens.len() {
        match sequence(&tokens, &mut pos, None, 0, false) {
            Ok(nodes) => {
                if !nodes.is_empty() { statements.push(Statement { nodes, guard: None }); }
            }
            Err(ParseFailure::Incomplete(e)) => return ParseStatus::Incomplete(e),
            Err(ParseFailure::Invalid(e)) => return ParseStatus::Invalid(e),
        }
    }
    ParseStatus::Complete(Parsed { statements })
}
