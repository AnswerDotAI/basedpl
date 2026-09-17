use crate::{Array, Element};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Default)]
pub(crate) struct Settings { pub enabled: bool, pub trees: bool, pub functions: bool }

impl Settings {
    pub fn interactive() -> Self { Self { enabled: true, trees: true, functions: true } }
    pub fn configure(&mut self, args: &str) -> Result<String, &'static str> {
        let mut next = *self;
        for arg in args.split_whitespace() {
            match arg.to_ascii_lowercase().as_str() {
                "on" => next.enabled = true,
                "off" => next.enabled = false,
                "?" => (),
                "reset" => next = Self::interactive(),
                "-style=max" => (),
                "-trains=tree" => next.trees = true,
                "-trains=def" => next.trees = false,
                "-fns" | "-fns=on" => next.functions = true,
                "-fns=off" => next.functions = false,
                _ => return Err("supported: ]box on|off|reset|? -style=max -trains=tree|def -fns=on|off"),
            }
        }
        *self = next;
        Ok(format!(
            "{} -style=max -trains={} -fns={}",
            if self.enabled { "ON" } else { "OFF" },
            if self.trees { "tree" } else { "def" },
            if self.functions { "on" } else { "off" }
        ))
    }
    pub fn array(self, a: &Array, inside: bool) -> String { if self.enabled && (!inside || self.functions) { diagram(a) } else { a.to_string() } }
}

struct Block { lines: Vec<String>, width: usize }
impl Block {
    fn new(text: String) -> Self {
        let lines: Vec<_> = text.split('\n').map(str::to_owned).collect();
        let width = lines.iter().map(|s| s.width()).max().unwrap_or(0);
        Self { lines, width }
    }
    fn line(&self, row: usize) -> String {
        let s = self.lines.get(row).map_or("", String::as_str);
        format!("{s}{}", " ".repeat(self.width - s.width()))
    }
    fn text(self) -> String { self.lines.join("\n") }
    fn framed(self, shape: &[usize], kind: char, padded: bool) -> Self {
        let pad = usize::from(padded);
        let width = (self.width + 2 * pad).max(shape.len().saturating_sub(1)).max(1);
        let top = shape.last().map_or('─', |&d| if d == 0 { '⊖' } else { '→' });
        let mut lines = vec![format!("┌{top}{}┐", "─".repeat(width - 1))];
        for row in 0..self.lines.len().max(shape.len().saturating_sub(1)) {
            let side = if row < shape.len().saturating_sub(1) { if shape[row] == 0 { '⌽' } else { '↓' } } else { '│' };
            let s = self.line(row);
            lines.push(format!("{side}{}{s}{}│", " ".repeat(pad), " ".repeat(width - self.width - pad)));
        }
        lines.push(format!("└{kind}{}┘", "─".repeat(width - 1)));
        Self { lines, width: width + 2 }
    }
}

fn array(a: &Array, budget: &mut usize) -> Block {
    // Render prototypes for empty axes. Cap diagrams independently of array storage.
    let shape: Vec<_> = a.shape().iter().map(|&d| d.max(1)).collect();
    let count = shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d)).unwrap_or(usize::MAX);
    if count > *budget { return Block::new(format!("… (shape {})", a.shape().iter().map(usize::to_string).collect::<Vec<_>>().join(" "))); }
    *budget -= count;
    let elements: Vec<_> = (0..count).map(|i| if a.is_empty() { a.prototype().clone() } else { a.at(i) }).collect();
    let nested = elements.iter().any(|e| matches!(e, Element::Nested(_)));
    let chars = elements.iter().all(|e| matches!(e, Element::Character(_)));
    let kind = if nested { '∊' } else if chars { '─' } else if elements.iter().all(|e| matches!(e, Element::Number(_))) { '~' } else { '+' };
    let cells: Vec<_> = elements
        .iter()
        .map(|e| match e {
            Element::Number(n) => Block::new(n.to_string()),
            Element::Character(c) => Block::new(if c.is_control() { c.escape_default().to_string() } else { c.to_string() }),
            Element::Nested(a) => array(a, budget),
        })
        .collect();
    let columns = shape.last().copied().unwrap_or(1);
    let mut widths = vec![0; columns];
    for (i, c) in cells.iter().enumerate() { widths[i % columns] = widths[i % columns].max(c.width); }
    let mut lines = Vec::new();
    for (row, chunk) in cells.chunks(columns).enumerate() {
        if row > 0 {
            let mut period = 1;
            for &dim in shape.iter().rev().skip(1).take(shape.len().saturating_sub(2)) {
                period *= dim;
                if row % period == 0 { lines.push(String::new()); }
            }
        }
        for y in 0..chunk.iter().map(|b| b.lines.len()).max().unwrap_or(1) {
            let mut line = String::new();
            for (x, cell) in chunk.iter().enumerate() {
                if x > 0 && !chars { line.push(' '); }
                let pad = " ".repeat(widths[x] - cell.width);
                if matches!(elements[row * columns + x], Element::Number(_)) { line.push_str(&pad); }
                line.push_str(&cell.line(y));
                if !matches!(elements[row * columns + x], Element::Number(_)) { line.push_str(&pad); }
            }
            lines.push(line);
        }
    }
    let block = Block::new(lines.join("\n"));
    if shape.is_empty() && !nested { block } else { block.framed(a.shape(), kind, nested) }
}

pub(crate) fn diagram(a: &Array) -> String { array(a, &mut 10_000).text() }

pub(crate) struct Tree { pub label: String, pub children: Vec<Tree> }
impl Tree {
    pub fn leaf(label: impl Into<String>) -> Self { Self { label: label.into(), children: vec![] } }
    pub fn render(&self) -> String {
        fn branch(node: &Tree, prefix: &str, edge: &str, next: &str, out: &mut Vec<String>) {
            out.push(format!("{prefix}{edge}{}", node.label.replace('\n', " ⋄ ")));
            let prefix = format!("{prefix}{next}");
            for (i, child) in node.children.iter().enumerate() {
                let last = i + 1 == node.children.len();
                branch(child, &prefix, if last { "└─ " } else { "├─ " }, if last { "   " } else { "│  " }, out);
            }
        }
        let mut out = Vec::new();
        branch(self, "", "", "", &mut out);
        out.join("\n")
    }
}
