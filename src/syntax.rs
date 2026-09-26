use crate::{
    primitive::{Hybrid, OperatorKind, Primitive},
    Error, ErrorKind, Number, Source, Span, Value,
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
    Guard(bool),
    Hybrid(Hybrid),
    Group(Vec<Node>),
    /// A run of nodes with no spaces between them, evaluated before its neighbours.
    Unit(Vec<Node>),
    /// Items of a bracketed list, or with `block` the major cells of an array. `record`: at least one item is `key:value`, and the items build one keyed vector.
    ArrayLiteral { cells: Vec<Vec<Node>>, block: bool, record: bool },
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
            NodeKind::Name(name) if name == "⍹" => DefinitionKind::DyadicOperator,
            NodeKind::Name(name) if name == "⍶" => DefinitionKind::MonadicOperator,
            NodeKind::Group(nodes) | NodeKind::Unit(nodes) => definition_kind(nodes),
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
    Pipe,
    BraceOpen,
    BraceClose,
    Literal(Value),
    Function(Primitive),
    Operator(OperatorKind),
    Open,
    Close,
    BracketOpen,
    BracketClose,
    Newline,
    Separator,
    Semicolon,
    Name(String),
    System(String),
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
    if chars.peek().is_some_and(|(_, c)| *c == '.') && chars.clone().nth(1).is_some_and(|(_, c)| c.is_ascii_digit()) {
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
                if chars.peek().is_some_and(|(_, c)| matches!(c, '.' | 'e' | 'E' | 'x' | 'ₓ' | 'r' | 'J' | 'j')) {
                    let &(i, c) = chars.peek().unwrap();
                    return Err(span(i + c.len_utf8()).error(ErrorKind::Syntax, "invalid complex numeric literal"));
                }
            }
            else if let Some(&(_, suffix @ ('x' | 'ₓ' | 'r'))) = chars.peek() {
                chars.next();
                if suffix == 'r' {
                    if chars.peek().is_some_and(|(_, c)| *c == '¯') { chars.next(); }
                    if digits(&mut chars) == 0 {
                        let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
                        return Err(span(end).error(ErrorKind::Syntax, "expected integer denominator"));
                    }
                }
                if chars.peek().is_some_and(|(_, c)| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | 'x' | 'ₓ' | 'r' | 'J' | 'j')) {
                    let &(i, c) = chars.peek().unwrap();
                    let end = i + c.len_utf8();
                    return Err(span(end).error(ErrorKind::Syntax, "invalid exact numeric literal"));
                }
            }
            let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
            let n = Number::parse(&source.text[start..end])
                .map_err(|k| span(end).error(k, "invalid numeric literal (real values, finite complex components or integer x/r components required)"))?;
            TokenKind::Literal(Value::scalar(n).unwrap())
        } else if (c.is_alphabetic() || matches!(c, '_' | '∆' | '⍙')) && Primitive::from_glyph(c).is_none() {
            chars.next();
            while chars.peek().is_some_and(|(_, c)| (c.is_alphanumeric() || matches!(c, '_' | '∆' | '⍙')) && Primitive::from_glyph(*c).is_none()) {
                chars.next();
            }
            let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
            TokenKind::Name(source.text[start..end].to_owned())
        } else {
            chars.next();
            match c {
                '\'' => {
                    let unclosed = |at| Err(span(at).error(ErrorKind::Syntax, "unclosed character literal"));
                    let c = match chars.next() { Some((_, c)) if c != '\n' => c, next => return unclosed(next.map_or(source.text.len(), |(i, _)| i)) };
                    match chars.peek() {
                        Some((_, '\'')) => {
                            chars.next();
                        }
                        None | Some((_, '\n')) => return unclosed(chars.peek().map_or(source.text.len(), |(i, _)| *i)),
                        Some(&(i, c)) => {
                            return Err(span(i + c.len_utf8()).error(ErrorKind::Syntax, "a character literal holds one character (strings use double quotes)"))
                        }
                    }
                    TokenKind::Literal(Value::Character(c))
                }
                '"' => {
                    let mut data = Vec::new();
                    loop {
                        match chars.next() {
                            Some((_, '"')) if chars.peek().is_some_and(|(_, c)| *c == '"') => {
                                chars.next();
                                data.push(Value::Character('"'));
                            }
                            Some((_, '"')) => break,
                            Some((_, '\n')) | None => {
                                return Err(span(chars.peek().map_or(source.text.len(), |(i, _)| *i)).error(ErrorKind::Syntax, "unclosed string"))
                            }
                            Some((_, c)) => data.push(Value::Character(c)),
                        }
                    }
                    TokenKind::Literal(Value::from_parts(vec![data.len()], data, Value::Character(' ')).unwrap())
                }
                '⍬' => TokenKind::Literal(Value::empty(vec![0], Value::Number(Number::try_from(0.0).unwrap())).unwrap()),
                '¨' => TokenKind::Operator(OperatorKind::Each),
                '⍨' => TokenKind::Operator(OperatorKind::Commute),
                '⊸' => TokenKind::Operator(OperatorKind::Before),
                '⍤' => TokenKind::Operator(OperatorKind::Rank),
                '⍠' => TokenKind::Operator(OperatorKind::Axis),
                '⍥' => TokenKind::Operator(OperatorKind::Over),
                '⟜' => TokenKind::Operator(OperatorKind::After),
                '∘' | '⍛' => {
                    return Err(span(start + c.len_utf8()).error(ErrorKind::Syntax, format!("{c} is retired: use ⊸ or ⟜ to bind or preprocess, and ⍤ for Atop")))
                }
                '.' => TokenKind::Operator(OperatorKind::Product),
                '⌝' => TokenKind::Operator(OperatorKind::Outer),
                '⌸' => TokenKind::Operator(OperatorKind::Key),
                '⍣' => TokenKind::Operator(OperatorKind::Power),
                '⇄' => TokenKind::Operator(OperatorKind::PairInverse),
                '⌾' => TokenKind::Operator(OperatorKind::Under),
                '∂' => TokenKind::Operator(OperatorKind::Differentiate),
                '◶' => TokenKind::Operator(OperatorKind::Agenda),
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
                '→' => TokenKind::Pipe,
                '•' if chars.peek().is_some_and(|(_, c)| c.is_alphanumeric() || *c == '_') => {
                    while chars.peek().is_some_and(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '∆' | '⍙')) { chars.next(); }
                    let end = chars.peek().map_or(source.text.len(), |(i, _)| *i);
                    let name = &source.text[start..end];
                    TokenKind::System(name.to_owned())
                }
                '⎕' => TokenKind::Output,
                ':' => {
                    let error = chars.peek().is_some_and(|(_, c)| *c == ':');
                    if error { chars.next(); }
                    TokenKind::Guard(error)
                }
                '/' | '⌿' | '\\' | '⍀' => TokenKind::Hybrid(Hybrid { scan: matches!(c, '\\' | '⍀'), first: matches!(c, '⌿' | '⍀') }),
                '{' => TokenKind::BraceOpen,
                '}' => TokenKind::BraceClose,
                '⍺' | '⍵' | '⍶' | '⍹' | '∇' | '⍢' => TokenKind::Name(c.to_string()),
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

fn invalid(span: &Span, message: &str) -> ParseFailure { ParseFailure::Invalid(span.error(ErrorKind::Syntax, message)) }

fn cover(nodes: &[Node]) -> Span { Span { source: nodes[0].span.source.clone(), range: nodes[0].span.range.start..nodes[nodes.len() - 1].span.range.end } }

/// Nodes and separators, before the enclosing delimiters give the separators their meaning.
enum Piece {
    Node(Node),
    Newline,
    Diamond(Span),
    Semicolon(Span),
}

struct Parser<'a> { tokens: &'a [Token], pos: usize }
impl Parser<'_> {
    fn pieces(&mut self, open: Option<&Token>, depth: usize) -> Result<Vec<Piece>, ParseFailure> {
        let mut pieces = Vec::new();
        while let Some(token) = self.tokens.get(self.pos) {
            self.pos += 1;
            let mut span = token.span.clone();
            let kind = match &token.kind {
                TokenKind::Newline => {
                    pieces.push(Piece::Newline);
                    continue;
                }
                TokenKind::Separator => {
                    pieces.push(Piece::Diamond(span));
                    continue;
                }
                TokenKind::Semicolon => {
                    pieces.push(Piece::Semicolon(span));
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
                    if !matched { return Err(invalid(&token.span, "mismatched closing delimiter")); }
                    return Ok(pieces);
                }
                TokenKind::Open | TokenKind::BracketOpen | TokenKind::BraceOpen => {
                    if depth == 128 { return Err(ParseFailure::Invalid(token.span.error(ErrorKind::Limit, "delimiters nested too deeply"))); }
                    let inner = self.pieces(Some(token), depth + 1)?;
                    span.range.end = self.tokens[self.pos - 1].span.range.end;
                    match token.kind {
                        TokenKind::BraceOpen => {
                            let statements = statements(inner)?.into_iter().map(statement).collect::<Result<Vec<_>, _>>()?;
                            let kind = statements.iter().map(|s| definition_kind(&s.nodes)).max().unwrap_or(DefinitionKind::Function);
                            NodeKind::Dfn(Arc::new(Definition { body: Parsed { statements }, span: span.clone(), kind }))
                        }
                        TokenKind::Open => NodeKind::Group(parenthesised(inner, &span)?),
                        _ => brackets(inner, &span)?,
                    }
                }
                TokenKind::Literal(a) => NodeKind::Literal(a.clone()),
                TokenKind::Function(f) => NodeKind::Function(*f),
                TokenKind::Operator(op) => NodeKind::Operator(*op),
                TokenKind::Name(name) => NodeKind::Name(name.clone()),
                TokenKind::System(name) => NodeKind::System(name.clone()),
                TokenKind::Assign => NodeKind::Assign,
                TokenKind::Pipe => NodeKind::Pipe,
                TokenKind::Output => NodeKind::Output,
                TokenKind::Guard(error) => {
                    if open.is_some_and(|o| matches!(o.kind, TokenKind::BraceOpen)) { NodeKind::Guard(*error) } else if !error { NodeKind::Function(Primitive::Keys) } else { return Err(invalid(&token.span, "error guards belong to dfns")); }
                }
                TokenKind::Hybrid(h) => NodeKind::Hybrid(h.clone()),
            };
            pieces.push(Piece::Node(Node { kind, span }));
        }
        if let Some(open) = open { return Err(ParseFailure::Incomplete(open.span.error(ErrorKind::Syntax, "unclosed delimiter"))); }
        Ok(pieces)
    }
}

/// Statements at the top level and in dfns: a line break or `⋄` ends one.
fn statements(pieces: Vec<Piece>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let (mut result, mut nodes) = (Vec::new(), Vec::new());
    for piece in pieces {
        match piece {
            Piece::Node(n) => nodes.push(n),
            Piece::Newline | Piece::Diamond(_) => {
                if !nodes.is_empty() { result.push(expression(std::mem::take(&mut nodes))?); }
            }
            Piece::Semicolon(s) => return Err(invalid(&s, "; separates items only inside brackets")),
        }
    }
    if !nodes.is_empty() { result.push(expression(nodes)?); }
    Ok(result)
}

/// Parentheses only group. A line break inside them is a space.
fn parenthesised(pieces: Vec<Piece>, span: &Span) -> Result<Vec<Node>, ParseFailure> {
    let mut nodes = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Node(n) => nodes.push(n),
            Piece::Newline => (),
            Piece::Diamond(s) => return Err(invalid(&s, "⋄ cannot separate items in parentheses: brackets write lists")),
            Piece::Semicolon(s) => return Err(invalid(&s, "; separates items only inside brackets")),
        }
    }
    if nodes.is_empty() { return Err(invalid(span, "empty grouping is not a value")); }
    expression(nodes)
}

/// Brackets build arrays. A space separates items, `;` separates items that contain spaces, and `⋄` separates major cells.
/// A line break is a space. Brackets round one item without `;` only group it.
fn brackets(pieces: Vec<Piece>, span: &Span) -> Result<NodeKind, ParseFailure> {
    let semicolon = pieces.iter().any(|p| matches!(p, Piece::Semicolon(_)));
    let diamond = pieces.iter().any(|p| matches!(p, Piece::Diamond(_)));
    if semicolon && diamond { return Err(invalid(span, "; and ⋄ cannot both separate items in one pair of brackets")); }
    let mut parts = vec![Vec::new()];
    for piece in pieces {
        match piece {
            Piece::Node(n) => parts.last_mut().unwrap().push(n),
            Piece::Newline => (),
            Piece::Diamond(_) | Piece::Semicolon(_) => parts.push(Vec::new()),
        }
    }
    // A trailing separator ends the last item or row.
    if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) { parts.pop(); }
    if (semicolon || diamond) && parts.iter().any(Vec::is_empty) {
        return Err(invalid(span, if semicolon { "empty item between semicolons" } else { "empty row between diamonds" }));
    }
    let record = |cells: &[Vec<Node>]| cells.iter().any(|c| keyed_item(c));
    if semicolon {
        let cells = parts.into_iter().map(expression).collect::<Result<Vec<_>, _>>()?;
        return Ok(NodeKind::ArrayLiteral { record: record(&cells), cells, block: false });
    }
    if diamond {
        let cells = parts
            .into_iter()
            .map(|row| {
                let span = cover(&row);
                let mut items = items(row)?;
                if items.len() == 1 { return Ok(items.pop().unwrap()); }
                Ok(vec![Node { kind: NodeKind::ArrayLiteral { record: record(&items), cells: items, block: false }, span }])
            })
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(NodeKind::ArrayLiteral { cells, block: true, record: false });
    }
    let mut cells = items(parts.pop().unwrap())?;
    let record = record(&cells);
    Ok(if cells.len() == 1 && !record { NodeKind::Group(cells.pop().unwrap()) } else { NodeKind::ArrayLiteral { cells, block: false, record } })
}

/// Items separated by spaces: each run of nodes with no space between them is one item.
fn items(nodes: Vec<Node>) -> Result<Vec<Vec<Node>>, ParseFailure> {
    let mut runs: Vec<Vec<Node>> = Vec::new();
    for node in nodes {
        match runs.last_mut() { Some(run) if run.last().unwrap().span.range.end == node.span.range.start => run.push(node), _ => runs.push(vec![node]) }
    }
    runs.into_iter().map(expression).collect()
}

/// An expression outside brackets, or one item between semicolons: literal runs, then units, then pipelines.
fn expression(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> { pipelines(units(literal_runs(nodes)?)?) }

/// Literals separated only by spaces form one vector literal.
fn literal_runs(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let (mut out, mut run) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if matches!(node.kind, NodeKind::Literal(_)) {
            run.push(node);
            continue;
        }
        literal_run(&mut run, &mut out)?;
        out.push(node);
    }
    literal_run(&mut run, &mut out)?;
    Ok(out)
}

fn literal_run(run: &mut Vec<Node>, out: &mut Vec<Node>) -> Result<(), ParseFailure> {
    if run.len() < 2 {
        out.append(run);
        return Ok(());
    }
    let span = cover(run);
    let items: Vec<_> = run.drain(..).map(|n| if let NodeKind::Literal(v) = n.kind { v } else { unreachable!() }).collect();
    let value = Value::new(vec![items.len()], items).map_err(|k| ParseFailure::Invalid(span.error(k, "invalid literal list")))?;
    out.push(Node { kind: NodeKind::Literal(value), span });
    Ok(())
}

/// Spaces separate units, and pipes and guards separate them too. The unit holding an assignment's target takes in the
/// `←` and its value, which runs to the next guard. The target stays flat within that unit, because assignment chooses
/// its own target. A unit alone between separators needs no wrapper.
fn units(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    let (mut result, mut part) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if matches!(node.kind, NodeKind::Guard(_)) {
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
        // A space before `←` leaves the target in the previous unit.
        if target.is_empty() && out.last().is_some_and(|n| !matches!(n.kind, NodeKind::Pipe)) {
            target = match out.pop() { Some(Node { kind: NodeKind::Unit(inner), .. }) => inner, Some(n) => vec![n], None => unreachable!() };
        }
        value.push_front(assign);
        for node in target.into_iter().rev() { value.push_front(node); }
        if out.is_empty() { continue; }
        let mut nodes = Vec::from(value);
        // Units before the target in its stage stay separate from it.
        if out.iter().rposition(|n| matches!(n.kind, NodeKind::Pipe)).map_or(0, |i| i + 1) < out.len() {
            let span = cover(&nodes);
            nodes = vec![Node { kind: NodeKind::Unit(nodes), span }];
        }
        out.extend(nodes);
        value = pipelines(out)?.into();
    }
    Ok(value.into())
}

/// Runs of nodes with no space between them become units, and pipes separate them. Returns the finished units and
/// the last run, which is still open.
fn spaced(nodes: Vec<Node>) -> (Vec<Node>, Vec<Node>) {
    let (mut out, mut run): (Vec<Node>, Vec<Node>) = (Vec::with_capacity(nodes.len()), Vec::new());
    for node in nodes {
        if matches!(node.kind, NodeKind::Pipe) {
            flush(&mut run, &mut out);
            out.push(node);
        } else {
            if run.last().is_some_and(|last| last.span.range.end != node.span.range.start) { flush(&mut run, &mut out); }
            run.push(node);
        }
    }
    (out, run)
}

fn flush(run: &mut Vec<Node>, out: &mut Vec<Node>) {
    if run.len() < 2 { out.append(run) } else {
        let span = cover(run);
        out.push(Node { kind: NodeKind::Unit(std::mem::take(run)), span });
    }
}

/// Close the last run. A unit alone between pipes needs no wrapper.
fn finish((mut out, mut run): (Vec<Node>, Vec<Node>)) -> Vec<Node> {
    flush(&mut run, &mut out);
    let (mut result, mut stage) = (Vec::with_capacity(out.len()), Vec::new());
    let close = |stage: &mut Vec<Node>, result: &mut Vec<Node>| {
        if let [Node { kind: NodeKind::Unit(inner), .. }] = stage.as_mut_slice() {
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

// Assignment encloses the pipeline; guards separate independent expressions.
fn pipelines(nodes: Vec<Node>) -> Result<Vec<Node>, ParseFailure> {
    if !nodes.iter().any(|n| matches!(n.kind, NodeKind::Pipe)) { return Ok(nodes); }
    let mut result = Vec::new();
    for part in nodes.split_inclusive(|n| matches!(n.kind, NodeKind::Guard(_))) {
        let (expression, guard) =
            if part.last().is_some_and(|n| matches!(n.kind, NodeKind::Guard(_))) { (&part[..part.len() - 1], part.last()) } else { (part, None) };
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

/// Whether a bracketed item is `key:value`: its first `:` follows keys, and no function comes before them.
pub(crate) fn keyed_item(nodes: &[Node]) -> bool {
    let Some(i) = nodes.iter().position(|n| matches!(n.kind, NodeKind::Function(Primitive::Keys))) else { return false; };
    let key = |n: &Node| matches!(n.kind, NodeKind::Literal(_) | NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. });
    i > 0 && nodes[..i].iter().all(key)
}

/// Check structure without evaluation. A complete input can still have a binding or domain error.
pub fn parse(source: Arc<Source>) -> ParseStatus {
    let tokens = match lex(&source) { Ok(tokens) => tokens, Err(e) => return ParseStatus::Invalid(e) };
    match (Parser { tokens: &tokens, pos: 0 }).pieces(None, 0).and_then(statements) {
        Ok(statements) => {
            ParseStatus::Complete(Parsed { statements: statements.into_iter().map(|nodes| Statement { nodes, kind: StatementKind::Expression }).collect() })
        }
        Err(ParseFailure::Incomplete(e)) => ParseStatus::Incomplete(e),
        Err(ParseFailure::Invalid(e)) => ParseStatus::Invalid(e),
    }
}
