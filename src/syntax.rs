use crate::{
    primitive::{Hybrid, OperatorKind, Primitive},
    Array, Element, Error, ErrorKind, Number, Source, Span,
};
use std::{iter::Peekable, str::CharIndices, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Literal(Array),
    Function(Primitive),
    Operator(OperatorKind),
    Name(String),
    Assign,
    Output,
    Guard(bool),
    Hybrid(Hybrid),
    Group(Vec<Node>),
    ArrayLiteral { cells: Vec<Vec<Node>>, block: bool },
    Selection(Vec<Vec<Node>>),
    Dfn(Arc<Definition>),
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
            NodeKind::ArrayLiteral { cells, .. } => cells.iter().map(|nodes| definition_kind(nodes)).max().unwrap_or(DefinitionKind::Function),
            NodeKind::Selection(cells) => cells.iter().map(|nodes| definition_kind(nodes)).max().unwrap_or(DefinitionKind::Function),
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
pub(crate) enum StatementKind { Expression, DefaultArgument, Guard { index: usize, error: bool } }

fn statement(nodes: Vec<Node>) -> Result<Statement, ParseFailure> {
    let mut guard = None;
    for (i, node) in nodes.iter().enumerate() {
        if let NodeKind::Guard(error) = node.kind {
            if i == 0 { return Err(ParseFailure::Invalid(node.span.error(ErrorKind::Syntax, "guard needs a condition"))); }
            if guard.is_some() { return Err(ParseFailure::Invalid(node.span.error(ErrorKind::Syntax, "a statement can contain only one guard"))); }
            guard = Some((i, error));
        }
    }
    let kind = if let Some((index, error)) = guard {
        StatementKind::Guard { index, error }
    } else if matches!(&nodes[0].kind, NodeKind::Name(name) if name == "⍺") && matches!(nodes.get(1).map(|n| &n.kind), Some(NodeKind::Assign)) {
        StatementKind::DefaultArgument
    } else { StatementKind::Expression };
    Ok(Statement { nodes, kind })
}

#[derive(Debug)]
pub enum ParseStatus { Complete(Parsed), Incomplete(Error), Invalid(Error) }

#[derive(Debug)]
enum TokenKind {
    BraceOpen,
    BraceClose,
    Literal(Array),
    Function(Primitive),
    Operator(OperatorKind),
    Open,
    Close,
    BracketOpen,
    BracketClose,
    Semicolon,
    Newline,
    Separator,
    Name(String),
    Assign,
    Output,
    Guard(bool),
    Hybrid(Hybrid),
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

fn lex(source: &Arc<Source>) -> Result<Vec<Token>, Error> {
    let mut chars = source.text.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some(&(start, c)) = chars.peek() {
        let span = |end| Span { source: source.clone(), range: start..end };
        let kind = if c.is_ascii_digit() || matches!(c, '¯' | '∞') || (c == '.' && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit())) {
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
                .map_err(|k| span(end).error(k, "invalid numeric literal (real values, finite complex components or integer x/r components required)"))?;
            TokenKind::Literal(Array::scalar(n).unwrap())
        } else if c.is_alphabetic() || matches!(c, '_' | '∆' | '⍙') {
            chars.next();
            while chars.peek().is_some_and(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '∆' | '⍙')) { chars.next(); }
            let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
            TokenKind::Name(source.text[start..end].to_owned())
        } else {
            chars.next();
            match c {
                '\'' => {
                    let mut data = Vec::new();
                    loop {
                        match chars.next() {
                            Some((_, '\'')) if chars.peek().is_some_and(|(_, c)| *c == '\'') => {
                                chars.next();
                                data.push(Element::Character('\''));
                            }
                            Some((_, '\'')) => break,
                            Some((_, '\n')) | None => {
                                return Err(span(chars.peek().map_or(source.text.len(), |(i, _)| *i)).error(ErrorKind::Syntax, "unclosed character literal"))
                            }
                            Some((_, c)) => data.push(Element::Character(c)),
                        }
                    }
                    let shape = if data.len() == 1 { vec![] } else { vec![data.len()] };
                    TokenKind::Literal(Array::from_parts(shape, data, Element::Character(' ')).unwrap())
                }
                '⍬' => TokenKind::Literal(Array::empty(vec![0], Element::Number(Number::try_from(0.0).unwrap())).unwrap()),
                '¨' => TokenKind::Operator(OperatorKind::Each),
                '⍨' => TokenKind::Operator(OperatorKind::Commute),
                '∘' => TokenKind::Operator(OperatorKind::Compose),
                '⍤' => TokenKind::Operator(OperatorKind::Rank),
                '⍥' => TokenKind::Operator(OperatorKind::Over),
                '⍛' => TokenKind::Operator(OperatorKind::Behind),
                '.' => TokenKind::Operator(OperatorKind::Product),
                '⌸' => TokenKind::Operator(OperatorKind::Key),
                '⍣' => TokenKind::Operator(OperatorKind::Power),
                '@' => TokenKind::Operator(OperatorKind::At),
                '⌺' => TokenKind::Operator(OperatorKind::Stencil),
                '(' => TokenKind::Open,
                ')' => TokenKind::Close,
                '[' => TokenKind::BracketOpen,
                ']' => TokenKind::BracketClose,
                ';' => TokenKind::Semicolon,
                '\n' => TokenKind::Newline,
                '⋄' => TokenKind::Separator,
                '←' => TokenKind::Assign,
                '⎕' if chars.peek().is_some_and(|(_, c)| c.is_alphabetic()) => {
                    while chars.peek().is_some_and(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '∆' | '⍙')) { chars.next(); }
                    let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
                    let name = &source.text[start..end];
                    match name.to_ascii_uppercase().as_str() {
                        "⎕A" | "⎕D" => {
                            let text = if name.eq_ignore_ascii_case("⎕A") { "ABCDEFGHIJKLMNOPQRSTUVWXYZ" } else { "0123456789" };
                            TokenKind::Literal(Array::new(vec![text.len()], text.chars().map(Element::Character).collect()).unwrap())
                        }
                        "⎕C" => TokenKind::Function(Primitive::Case),
                        "⎕UCS" => TokenKind::Function(Primitive::Unicode),
                        _ => return Err(span(end).error(ErrorKind::Unsupported, format!("{name} is not supported yet"))),
                    }
                }
                '⎕' => TokenKind::Output,
                ':' => {
                    let error = chars.peek().is_some_and(|(_, c)| *c == ':');
                    if error { chars.next(); }
                    TokenKind::Guard(error)
                }
                '/' | '⌿' | '\\' | '⍀' => TokenKind::Hybrid(Hybrid { scan: matches!(c, '\\' | '⍀'), first: matches!(c, '⌿' | '⍀'), axis: None }),
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

struct Parser<'a> { tokens: &'a [Token], pos: usize }
impl Parser<'_> {
    fn expressions(&mut self, open: Option<&Token>, depth: usize) -> Result<(Vec<Vec<Node>>, bool), ParseFailure> {
        let (mut pieces, mut nodes, mut separated, mut indexed) = (Vec::new(), Vec::new(), false, false);
        while let Some(token) = self.tokens.get(self.pos) {
            self.pos += 1;
            let kind = match &token.kind {
                TokenKind::Newline | TokenKind::Separator => {
                    if indexed { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "cannot mix index and literal separators"))); }
                    separated = true;
                    if !nodes.is_empty() { pieces.push(std::mem::take(&mut nodes)); }
                    continue;
                }
                TokenKind::Semicolon => {
                    if separated || !open.is_some_and(|o| matches!(o.kind, TokenKind::BracketOpen)) {
                        return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "semicolon belongs to index brackets")));
                    }
                    indexed = true;
                    pieces.push(std::mem::take(&mut nodes));
                    continue;
                }
                TokenKind::Close | TokenKind::BracketClose | TokenKind::BraceClose => {
                    let matched = open.is_some_and(|o| {
                        matches!(
                            (&o.kind, &token.kind),
                            (TokenKind::Open, TokenKind::Close)
                                | (TokenKind::BracketOpen, TokenKind::BracketClose)
                                | (TokenKind::BraceOpen, TokenKind::BraceClose)
                        )
                    });
                    if !matched { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "mismatched closing delimiter"))); }
                    if indexed || !nodes.is_empty() { pieces.push(nodes); }
                    return Ok((pieces, separated));
                }
                TokenKind::Open | TokenKind::BracketOpen | TokenKind::BraceOpen => {
                    if depth == 128 { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Limit, "delimiters nested too deeply"))); }
                    let (mut cells, separated) = self.expressions(Some(token), depth + 1)?;
                    let span = Span { source: token.span.source.clone(), range: token.span.range.start..self.tokens[self.pos - 1].span.range.end };
                    let kind = if matches!(token.kind, TokenKind::BraceOpen) {
                        let statements = cells.into_iter().map(statement).collect::<Result<Vec<_>, _>>()?;
                        let kind = statements.iter().map(|s| definition_kind(&s.nodes)).max().unwrap_or(DefinitionKind::Function);
                        NodeKind::Dfn(Arc::new(Definition { body: Parsed { statements }, span: span.clone(), kind }))
                    } else if !separated && matches!(token.kind, TokenKind::BracketOpen) { NodeKind::Selection(cells) } else if cells.is_empty() {
                        return Err(ParseFailure::Invalid(
                            span.error(ErrorKind::Unsupported, "empty delimiters are not an array literal; namespaces are unsupported"),
                        ));
                    } else if separated {
                        NodeKind::ArrayLiteral { cells, block: matches!(token.kind, TokenKind::BracketOpen) }
                    } else if matches!(token.kind, TokenKind::Open) { NodeKind::Group(cells.pop().unwrap()) } else { unreachable!() };
                    nodes.push(Node { kind, span });
                    continue;
                }
                TokenKind::Literal(a) => NodeKind::Literal(a.clone()),
                TokenKind::Function(f) => NodeKind::Function(*f),
                TokenKind::Operator(op) => NodeKind::Operator(*op),
                TokenKind::Name(name) => NodeKind::Name(name.clone()),
                TokenKind::Assign => NodeKind::Assign,
                TokenKind::Output => NodeKind::Output,
                TokenKind::Guard(error) => {
                    if !open.is_some_and(|o| matches!(o.kind, TokenKind::BraceOpen)) {
                        return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Syntax, "guards belong to dfn statements; namespaces are unsupported")));
                    }
                    NodeKind::Guard(*error)
                }
                TokenKind::Hybrid(h) => NodeKind::Hybrid(*h),
            };
            nodes.push(Node { kind, span: token.span.clone() });
        }
        if let Some(open) = open { return Err(ParseFailure::Incomplete(open.span.error(ErrorKind::Syntax, "unclosed delimiter"))); }
        if !nodes.is_empty() { pieces.push(nodes); }
        Ok((pieces, separated))
    }
}

/// Check structure without evaluation. A complete input can still have a binding or domain error.
pub fn parse(source: Arc<Source>) -> ParseStatus {
    let tokens = match lex(&source) { Ok(tokens) => tokens, Err(e) => return ParseStatus::Invalid(e) };
    match (Parser { tokens: &tokens, pos: 0 }).expressions(None, 0) {
        Ok((pieces, _)) => {
            ParseStatus::Complete(Parsed { statements: pieces.into_iter().map(|nodes| Statement { nodes, kind: StatementKind::Expression }).collect() })
        }
        Err(ParseFailure::Incomplete(e)) => ParseStatus::Incomplete(e),
        Err(ParseFailure::Invalid(e)) => ParseStatus::Invalid(e),
    }
}
