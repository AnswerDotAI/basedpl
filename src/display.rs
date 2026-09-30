use crate::{
    array::{page_breaks, Decimals},
    Value,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Default)]
pub(crate) struct Settings { pub enabled: bool, pub trees: bool, pub functions: bool }

impl Settings {
    pub fn interactive() -> Self { Self { enabled: true, trees: true, functions: true } }
    pub fn configure(&mut self, args: &str, defaults: Self) -> Result<String, &'static str> {
        let mut next = *self;
        for arg in args.split_whitespace() {
            match arg.to_ascii_lowercase().as_str() {
                "on" => next.enabled = true,
                "off" => next.enabled = false,
                "?" => (),
                "reset" => next = defaults,
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
    pub fn array(self, a: &Value, inside: bool) -> String { if self.enabled && (!inside || self.functions) { diagram(a) } else { a.to_string() } }
}

struct Block { lines: Vec<String>, width: usize }
impl Block {
    fn new(text: String) -> Self {
        let lines: Vec<_> = text.split('\n').map(str::to_owned).collect();
        let width = lines.iter().map(|s| s.width()).max().unwrap_or(0);
        Self { lines, width }
    }
    fn line(&self, row: usize) -> String { let s = self.lines.get(row).map_or("", String::as_str); format!("{s}{}", " ".repeat(self.width - s.width())) }
    fn text(self) -> String { self.lines.join("\n") }
    fn labelled(self, name: &str) -> Self {
        let label = format!("{name}:");
        let pad = " ".repeat(label.width());
        let lines = self.lines.iter().enumerate().map(|(i, s)| format!("{}{s}", if i == 0 { &label } else { &pad })).collect();
        Self { lines, width: self.width + label.width() }
    }
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

/// The bottom-edge marker for an array's storage.
fn marker(a: &Value) -> char {
    match a.storage_name() {
        "boolean" => '$',
        "integer" => 'ₓ',
        "float" => '~',
        "complex" => 'j',
        "character" => '─',
        _ => '+',
    }
}

fn array(a: &Value, budget: &mut usize) -> Block {
    if a.is_unit() || (a.has_keys() && a.shape() == [0]) { return Block::new(a.to_string()); }
    // Render prototypes for empty axes. Cap diagrams independently of array storage.
    let shape: Vec<_> = a.shape().iter().map(|&d| d.max(1)).collect();
    let count = shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d)).unwrap_or(usize::MAX);
    if count > *budget { return Block::new(format!("… (shape {})", a.shape().iter().map(usize::to_string).collect::<Vec<_>>().join(" "))); }
    *budget -= count;
    if a.has_keys() && a.shape().len() > 1 { return Block::new(a.to_string()).framed(a.shape(), marker(a), false); }
    let elements: Vec<_> = (0..count).map(|i| if a.is_empty() { a.prototype().clone() } else { a.at(i) }).collect();
    let nested = elements.iter().any(|e| matches!(e, Value::Array(_)));
    let chars = elements.iter().all(|e| matches!(e, Value::Character(_)));
    let kind = if nested { '∊' } else { marker(a) };
    let cells: Vec<_> = elements
        .iter()
        .map(|e| match e {
            // An `ₓ` box already says its numbers are exact.
            Value::Number(n) if kind == 'ₓ' => Block::new(format!("{n:#}")),
            Value::Number(n) => Block::new(n.to_string()),
            Value::Character(c) => Block::new(if c.is_control() { c.escape_default().to_string() } else { c.to_string() }),
            a @ Value::Array(_) => array(a, budget),
            Value::Function(f) => Block::new(f.to_string()),
        })
        .enumerate()
        .map(|(i, cell)| match a.keys(0).and_then(|keys| keys.names().get(i)?.as_ref()) { Some(key) => cell.labelled(key), None => cell })
        .collect();
    let columns = shape.last().copied().unwrap_or(1);
    // Numbers in a column line up on their decimal points, as they do in plain display.
    let numbers: Vec<_> = cells.iter().zip(&elements).map(|(c, e)| matches!(e, Value::Number(_)).then(|| Decimals::of(c.lines[0].chars()))).collect();
    let (mut widths, mut decimals) = (vec![0; columns], vec![Decimals::default(); columns]);
    for (i, (c, n)) in cells.iter().zip(&numbers).enumerate() {
        widths[i % columns] = widths[i % columns].max(c.width);
        if let Some(n) = n { decimals[i % columns].fit(*n); }
    }
    for (w, d) in widths.iter_mut().zip(&decimals) { *w = (*w).max(d.width()); }
    let mut lines = Vec::new();
    for (row, chunk) in cells.chunks(columns).enumerate() {
        lines.extend(std::iter::repeat_n(String::new(), page_breaks(&shape, row)));
        for y in 0..chunk.iter().map(|b| b.lines.len()).max().unwrap_or(1) {
            let mut line = String::new();
            for (x, cell) in chunk.iter().enumerate() {
                if x > 0 && !(chars && !a.has_keys()) { line.push(' '); }
                let (before, after) = match numbers[row * columns + x] {
                    Some(n) => {
                        let (before, after) = decimals[x].padding(n);
                        (before + widths[x] - decimals[x].width(), after)
                    }
                    None => (0, widths[x] - cell.width),
                };
                line.extend(std::iter::repeat_n(' ', before));
                line.push_str(&cell.line(y));
                line.extend(std::iter::repeat_n(' ', after));
            }
            lines.push(line);
        }
    }
    let block = Block::new(lines.join("\n"));
    block.framed(a.shape(), kind, nested)
}

pub(crate) fn diagram(a: &Value) -> String { array(a, &mut 10_000).text() }

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

/// Convert a keyed vector from MIME types to text into the bundle sent to frontends.
pub(crate) fn bundle(value: &Value) -> Result<crate::MimeBundle, crate::ErrorKind> {
    crate::keyed::pairs(value)?
        .into_iter()
        .map(|(name, value)| match crate::keyed::name(&value) {
            Some(text) if name.contains('/') && !name.contains(char::is_whitespace) => Ok((name.to_string(), text.to_string())),
            _ => Err(crate::ErrorKind::Domain),
        })
        .collect()
}

/// `record` with a `_mime` field holding the native renderer `render`.
pub(crate) fn with_renderer(
    record: &Value,
    name: &'static str,
    render: fn(Option<&Value>, &Value, &crate::execution::Context<'_>) -> Result<Value, crate::Error>,
) -> Result<Value, crate::ErrorKind> {
    let renderer = crate::system::native(name, crate::system::Call::Value(render), crate::system::Valence::Ambivalent);
    crate::keyed::merge(record, &crate::keyed::vector(vec!["_mime".into()], vec![renderer])?)
}

/// A MIME bundle holding SVG text.
pub(crate) fn svg(text: Value) -> Result<Value, crate::ErrorKind> { crate::keyed::vector(vec!["image/svg+xml".into()], vec![text]) }
