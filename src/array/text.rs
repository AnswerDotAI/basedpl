//! Text forms of values: `⍕` formatting, and source text that reads back.

use super::*;

/// Text as a double-quoted string literal.
pub(crate) fn quoted(text: &str) -> String { format!("\"{}\"", text.replace('"', "\"\"")) }

/// Whether `test` holds for a character of source text outside brackets, parentheses, braces and quotes.
fn outside(text: &str, test: impl Fn(char) -> bool) -> bool {
    let (mut depth, mut chars) = (0, text.chars());
    while let Some(c) = chars.next() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            // A string ends at a quote that isn't doubled.
            '"' => {
                while let Some(c) = chars.next() {
                    if c != '"' { continue; }
                    if chars.clone().next() != Some('"') { break; }
                    chars.next();
                }
            }
            // A character literal is one character between quotes, which may itself be a quote or a space.
            '\'' => {
                chars.nth(1);
            }
            c if depth == 0 && test(c) => return true,
            _ => (),
        }
    }
    false
}

/// Whether source text has a space or a `:` outside brackets, parentheses, braces and quotes.
fn needs_group(text: &str) -> bool { outside(text, |c| c == ' ' || c == ':') }

impl Value {
    pub(crate) fn formatted(&self) -> Result<Self, ErrorKind> {
        if self.is_unit() {
            if let Value::Function(f) = self.at(0) {
                let text = f.to_string();
                return Self::characters(vec![text.chars().count()], text.chars().collect());
            }
        }
        // A keyed array formats as its display text, so its keys stay visible.
        let keyed = self.has_keys();
        if !keyed && matches!(self.prototype(), Value::Character(_)) && self.elements().all(|e| matches!(e, Value::Character(_))) { return Ok(self.clone()); }
        let numeric = matches!(self.prototype(), Value::Number(_)) && self.all_numbers();
        if !numeric && !keyed { return self.formatted_cells(); }
        let (shape, text) = if !keyed && self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            // One line per row, without blank lines between planes, as the result keeps the leading axes.
            let rows = crate::display::rows(self.shape(), None, None)
                .into_iter()
                .map(|(_, _, spots)| {
                    (0, spots.iter().map(|spot| crate::display::Cell::spot(spot, |i| crate::display::Cell::number(self.at(i).to_string()))).collect())
                })
                .collect();
            let (width, lines) = crate::display::grid(columns, rows, crate::display::Style::Uniform { spaced: true, right: true });
            let mut shape = self.shape().to_vec();
            *shape.last_mut().unwrap() = width;
            (shape, lines.concat().concat())
        } else if !keyed && self.len() >= 2 && matches!(self.as_items(), Items::Integers(_) | Items::Extended(_)) {
            // An exact vector writes as it displays, with one `ₓ` after its brackets.
            let text = self.literal();
            (vec![text.chars().count()], text)
        } else if !keyed {
            // Other numbers join with spaces, whatever form the value displays in.
            use fmt::Write;
            let mut text = String::new();
            for (i, e) in self.elements().enumerate() {
                if i > 0 { text.push(' '); }
                let Value::Number(n) = e else { unreachable!("every item is a number") };
                write!(text, "{n}").unwrap();
            }
            (vec![text.chars().count()], text)
        } else {
            let text = self.to_string();
            let lines: Vec<_> = text.split('\n').collect();
            if lines.len() == 1 { (vec![text.chars().count()], text) } else {
                let width = lines.iter().map(|s| s.chars().count()).max().unwrap();
                let joined = lines.iter().map(|s| format!("{s}{}", " ".repeat(width - s.chars().count()))).collect();
                (vec![lines.len(), width], joined)
            }
        };
        generated_len(&shape)?;
        let mut chars = Vec::with_capacity(text.chars().count());
        chars.extend(text.chars());
        Self::characters(shape, chars)
    }

    fn formatted_cells(&self) -> Result<Self, ErrorKind> {
        use crate::display::{grid, Cell, Style};
        if self.is_empty() { return Self::empty(vec![0], Value::Character(' ')); }
        let rank = self.shape().len();
        let columns = self.shape().last().copied().unwrap_or(1);
        let (mut size, mut matrix, mut rows) = (0, rank > 1, Vec::new());
        for start in (0..self.len()).step_by(columns) {
            let mut cells = Vec::with_capacity(columns);
            for item in self.items(start..start + columns) {
                let text = item.clone().formatted()?;
                size += text.len();
                generated_len(&[size])?;
                matrix |= text.shape().len() > 1;
                cells.push(Cell::formatted(text.formatted_rows()?, &item));
            }
            rows.push((0, cells));
        }
        let (width, rows) = grid(columns, rows, Style::PerColumn);
        // Each matrix of cells is one plane of lines. A higher rank keeps its leading axes, and every plane is as tall as the tallest.
        let planes: Vec<Vec<String>> = rows.chunks(if rank > 2 { self.shape()[rank - 2] } else { rows.len() }).map(<[_]>::concat).collect();
        let height = planes.iter().map(Vec::len).max().unwrap();
        let shape = if rank > 2 { [&self.shape()[..rank - 2], &[height, width]].concat() } else if matrix { vec![height, width] } else { vec![width] };
        let mut chars = Vec::with_capacity(generated_len(&shape)?);
        for plane in planes {
            chars.extend(plane.iter().flat_map(|line| line.chars()));
            chars.extend(std::iter::repeat_n(' ', (height - plane.len()) * width));
        }
        Self::characters(shape, chars)
    }

    /// The rows of a character array, with a blank row before each one that starts a new plane.
    fn formatted_rows(&self) -> Result<Vec<Vec<char>>, ErrorKind> {
        let columns = self.shape().last().copied().unwrap_or(1);
        generated_len(&[self.shape().iter().rev().skip(1).product(), columns.max(1)])?;
        let char_at = |i| match self.at(i) { Value::Character(c) => c, _ => unreachable!("a formatted array holds characters") };
        let mut lines = Vec::new();
        for (breaks, _, spots) in crate::display::rows(self.shape(), None, None) {
            lines.extend(std::iter::repeat_n(vec![' '; columns], breaks));
            lines.push(
                spots
                    .iter()
                    .map(|spot| match *spot { crate::display::Spot::At(i) => char_at(i), crate::display::Spot::Gap(c) => c })
                    .collect(),
            );
        }
        Ok(lines)
    }

    /// Source text that reads back as the value. Strings are quoted, and arrays use bracket notation.
    pub fn literal(&self) -> String { self.source(Elide::NONE) }

    /// Whether the value is a function or an operator, or holds one at any depth, including as an empty array's prototype.
    pub(crate) fn holds_function(&self) -> bool {
        match self {
            Self::Function(_) | Self::Operator(_) => true,
            Self::Array(_) => match self.as_items() {
                Items::Values(items) => items.iter().any(Self::holds_function) || (items.is_empty() && self.prototype().holds_function()),
                _ => false,
            },
            _ => false,
        }
    }

    /// Source text for the value, with large arrays elided as `el` says. Elided text doesn't read back.
    pub(crate) fn source(&self, el: Elide) -> String {
        match self {
            Self::Number(n) => el.number(n),
            Self::Character(c) if c.is_control() => format!("•ucs {}", *c as u32),
            Self::Character(c) => format!("'{c}'"),
            Self::Function(f) => {
                let text = f.bpl();
                // A native function with no source spelling, such as a generator's `roll`, can't read back.
                if f.system_call().is_some() && crate::system::lookup(&text).is_none() { return f.to_string(); }
                if text.contains(' ') { format!("({text})") } else { text }
            }
            Self::Operator(op) => {
                let text = op.to_string();
                if text.contains(' ') { format!("({text})") } else { text }
            }
            Self::Array(_) => {
                if self.axis_names().iter().any(Option::is_some) { return self.named_literal(el); }
                let edges = if self.shape().len() == 1 { el.last(self.shape()) } else { el.edges(self.shape()) };
                if self.string_literal().is_some() { return quoted(&self.elided_text(edges)); }
                if let Some(s) = crate::keyed::name(self) {
                    return format!(",•ucs {}", s.chars().map(|c| (c as u32).to_string()).collect::<Vec<_>>().join(" "));
                }
                if self.is_empty() && !self.has_keys() { return self.empty_literal(el); }
                let exact = self.marks_exact();
                match self.shape().len() {
                    0 => Self::enclosed_literal(&self.at(0), el),
                    1 if exact => format!("[{}]ₓ", self.bracket_items(Self::unmarked_item, el, edges)),
                    1 => self.vector_literal(el, edges),
                    _ if self.has_keys() => self.keyed_literal(el),
                    _ if exact => format!("{}ₓ", self.block_literal(Self::unmarked_item, el, edges)),
                    _ => self.block_literal(Self::item, el, edges),
                }
            }
        }
    }

    /// The items display shows of a vector, with `None` for the gap that `edges` leaves.
    fn shown(&self, edges: Option<usize>) -> impl Iterator<Item = Option<Value>> + '_ { positions(self.len(), edges).map(|i| i.map(|i| self.at(i))) }

    /// A character vector's characters, with `…` for the gap that `edges` leaves.
    pub(crate) fn elided_text(&self, edges: Option<usize>) -> String {
        self.shown(edges)
            .map(|c| match c { Some(Self::Character(c)) => c, _ => '…' })
            .collect()
    }

    /// Text for each shown item of a vector, written by `item`, with `…` for the gap that `edges` leaves.
    pub(crate) fn shown_items(&self, edges: Option<usize>, item: impl Fn(Value) -> String) -> Vec<String> {
        self.shown(edges).map(|e| e.map_or_else(|| "…".into(), &item)).collect()
    }

    /// An array of rank 2 or more in array notation, one major cell at a time, with `item` writing each item. `edges`
    /// elides positions on every axis of the whole array.
    fn block_literal(&self, item: fn(&Self, Elide) -> String, el: Elide, edges: Option<usize>) -> String {
        let Ok(cells) = self.cells(self.shape().len() - 1) else { return crate::display::plain(self, el) };
        let rows: Result<Vec<String>, ErrorKind> =
            positions(cells.len(), edges).map(|i| i.map_or(Ok("…".into()), |i| Ok(cells.get(i)?.row(item, el, edges)))).collect();
        match rows {
            // One major cell needs a trailing `⋄`, or it reads back as that cell alone.
            Ok(rows) if rows.len() == 1 => format!("[{}⋄]", rows[0]),
            Ok(rows) => format!("[{}]", rows.join("⋄")),
            Err(_) => crate::display::plain(self, el),
        }
    }

    /// Whether the value is a nonempty array in integer storage, which writes one `ₓ` after its notation in place of one for
    /// each number.
    fn marks_exact(&self) -> bool { !self.is_empty() && matches!(self.as_items(), Items::Integers(_) | Items::Extended(_)) }
    /// An item of integer storage without its `ₓ`. One `ₓ` after the whole notation marks every number exact.
    fn unmarked_item(&self, _: Elide) -> String { let Self::Number(n) = self else { unreachable!("integer storage holds numbers") }; format!("{n:#}") }
    /// The value as one item inside brackets. Text with a space between its runs, or with a `:` that would read as a key,
    /// needs parentheses.
    pub(crate) fn item(&self, el: Elide) -> String { let text = self.source(el); if needs_group(&text) { format!("({text})") } else { text } }
    /// The value as an operator's operand, which is one item. Text with anything other than literals outside brackets, as in
    /// `⊂3` or `0 3⍴0`, needs parentheses.
    pub(crate) fn operand(&self) -> String {
        let text = self.literal();
        if outside(&text, |c| !(c.is_ascii_digit() || " ¯.∞⍬".contains(c) || crate::syntax::subscript(c))) { format!("({text})") } else { text }
    }

    /// Source text for a scalar holding `content`: `ᵘ` after a function, and `⊂` before anything else.
    fn enclosed_literal(content: &Value, el: Elide) -> String {
        match content { Self::Function(f) => f.superscripted("ᵘ", &mut 1000), _ => format!("⊂{}", content.item(el)) }
    }

    /// An array with named axes: its keyed shape reshapes the array without names. Reshape keeps position keys.
    fn named_literal(&self, el: Elide) -> String {
        let shape: Vec<_> =
            self.axis_names().iter().zip(self.shape()).map(|(n, len)| n.as_ref().map_or_else(|| len.to_string(), |n| format!("{}:{len}", quoted(n)))).collect();
        format!("[{}]⍴{}", shape.join(" "), self.clone().with_axis_names(vec![]).unwrap().source(el))
    }

    /// An array of rank 2 or more with keys: a key list for each axis, applied to the array without keys. A position
    /// stands for each missing key.
    fn keyed_literal(&self, el: Elide) -> String {
        let edges = el.edges(self.shape());
        let lists: Vec<_> = self
            .shape()
            .iter()
            .enumerate()
            .map(|(axis, &len)| {
                let keys: Vec<_> = positions(len, edges)
                    .map(|i| i.map_or_else(|| "…".into(), |i| self.keys(axis).and_then(|k| k.names()[i].as_ref()).map_or_else(|| i.to_string(), |k| quoted(k))))
                    .collect();
                match keys.len() { 0 => "⍬".into(), 1 => format!("[{}]", keys[0]), _ => keys.join(" ") }
            })
            .collect();
        format!("[{}]:{}", lists.join(";"), self.unkeyed().source(el))
    }

    /// Whether plain display shows the vector as a strand: a vector of two or more numbers, characters or strings. Exact
    /// integers show in brackets instead.
    pub(crate) fn is_strand(&self) -> bool {
        matches!(self, Self::Array(_))
            && self.shape().len() == 1
            && self.len() >= 2
            && !self.has_keys()
            && self.axis_names().iter().all(Option::is_none)
            && self.string_literal().is_none()
            && match self.as_items() {
                Items::Values(items) => {
                    items.iter().all(|e| matches!(e, Self::Number(_)) || matches!(e, Self::Character(c) if !c.is_control()) || e.string_literal().is_some())
                }
                Items::Characters(cs) => cs.iter().all(|c| !c.is_control()),
                Items::Integers(_) | Items::Extended(_) => false,
                _ => true,
            }
    }

    /// An empty array: `⍬` or `""` for a simple vector, and otherwise its shape reshaping its prototype.
    fn empty_literal(&self, el: Elide) -> String {
        let prototype = self.prototype();
        match (self.shape(), &prototype) {
            ([_], Self::Number(n)) if n.as_bool().is_some() => "0⍴$f".into(),
            ([_], Self::Number(n)) if n.is_exact() => "⍬ₓ".into(),
            ([_], Self::Number(_)) => "⍬".into(),
            ([_], Self::Character(_)) => "\"\"".into(),
            (shape, _) => {
                let shape: Vec<_> = shape.iter().map(ToString::to_string).collect();
                let fill = if matches!(prototype, Self::Array(_)) { Self::enclosed_literal(&prototype, el) } else { prototype.source(el) };
                format!("{}⍴{fill}", shape.join(" "))
            }
        }
    }
    fn vector_literal(&self, el: Elide, edges: Option<usize>) -> String {
        // An empty record keeps its keyed axis, which `[]` would lose.
        if self.keys(0).is_some() && self.is_empty() { return "⍬:⍬".into(); }
        if self.keys(0).is_none() && self.len() >= 2 && self.elements().all(|e| e.is_row()) {
            let exact = self.elements().all(|e| e.marks_exact());
            let item = if exact { Self::unmarked_item } else { Self::item };
            let rows = self.shown_items(edges, |e| e.bracket_items(item, el, el.edges(e.shape()))).join("⋄");
            return if exact { format!("({rows})ₓ") } else { format!("({rows})") };
        }
        format!("[{}]", self.bracket_items(Self::item, el, edges))
    }

    /// A vector's items as they appear between brackets, each written by `item`, with `…` for the gap that `edges` leaves.
    fn bracket_items(&self, item: fn(&Self, Elide) -> String, el: Elide, edges: Option<usize>) -> String {
        let key = |i: usize| self.keys(0).and_then(|k| k.names()[i].clone());
        let items: Vec<_> = positions(self.len(), edges)
            .map(|i| i.map_or_else(|| "…".into(), |i| key(i).map_or_else(|| item(&self.at(i), el), |k| format!("{}:{}", quoted(&k), item(&self.at(i), el)))))
            .collect();
        items.join(" ")
    }

    /// Whether the value prints as one row of `(a⋄b)`: a vector with items, other than a string.
    fn is_row(&self) -> bool {
        matches!(self, Self::Array(_))
            && self.shape().len() == 1
            && !self.is_empty()
            && crate::keyed::name(self).is_none()
            && self.axis_names().iter().all(Option::is_none)
    }

    /// A major cell as one part of array notation: a vector's items side by side, a higher rank in its own notation, or one
    /// item. A row with one item needs its own brackets, because a single item would be a cell by itself. `edges` elides the
    /// whole array's positions.
    fn row(&self, item: fn(&Self, Elide) -> String, el: Elide, edges: Option<usize>) -> String {
        if self.shape().len() == 1 && !self.has_keys() && self.string_literal().is_none() {
            if self.len() == 1 { return format!("[{}]", item(&self.at(0), el)); }
            self.shown_items(edges, |e| item(&e, el)).join(" ")
        } else if self.shape().len() > 1 && !self.has_keys() { self.block_literal(item, el, edges) } else { item(self, el) }
    }

    /// A character vector as a double-quoted literal, or `None` when it holds a control character other than a newline.
    fn string_literal(&self) -> Option<String> {
        if let Self::Array(_) = self { crate::keyed::name(self).filter(|s| !s.chars().any(|c| c.is_control() && c != '\n')).map(|s| quoted(&s)) } else { None }
    }
}
