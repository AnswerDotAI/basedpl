use crate::{keyed, ErrorAt, Number, Value};
use unicode_width::UnicodeWidthStr;

/// A column of numbers aligned on their decimal points: its widest text before the point, and its widest from the point on.
#[derive(Clone, Copy, Default)]
struct Decimals { left: usize, right: usize }
impl Decimals {
    /// The characters of a number's text before its decimal point, and from the point on.
    fn of(text: impl IntoIterator<Item = char>) -> Self {
        let mut d = Self::default();
        for c in text { if d.right > 0 || c == '.' { d.right += 1 } else { d.left += 1 } }
        d
    }
    /// Widens the column to hold `number`.
    fn fit(&mut self, number: Self) { *self = Self { left: self.left.max(number.left), right: self.right.max(number.right) } }
    fn width(self) -> usize { self.left + self.right }
    /// The spaces before and after `number` that align it in the column.
    fn padding(self, number: Self) -> (usize, usize) { (self.left - number.left, self.right - number.right) }
}

/// How much of a large array or a long float display shows. An array of more than `limit` items shows the first and last `edges`
/// positions of each axis longer than twice `edges`, and a marker for the positions between. A float shows `prec` significant digits.
/// `width` is the widest line display shows. `columns`, at most `edges` when that applies, is how many positions display keeps at
/// each end of a last axis to fit `width`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Elide {
    pub limit: usize,
    pub edges: usize,
    pub prec: usize,
    pub width: usize,
    pub columns: usize,
}
impl Default for Elide { fn default() -> Self { Self { limit: 1000, edges: 3, ..Self::NONE } } }
impl Elide {
    /// Shows every item and every digit, as source text and `⍕` do.
    pub const NONE: Self = Self { limit: usize::MAX, edges: 0, prec: usize::MAX, width: usize::MAX, columns: usize::MAX };
    /// The positions kept at each end of a long axis of an array of `shape`, or `None` when display shows every item.
    pub fn edges(self, shape: &[usize]) -> Option<usize> {
        let count = shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d)).unwrap_or(usize::MAX);
        (count > self.limit).then_some(self.edges)
    }
    /// The positions kept at each end of the last axis of an array of `shape`, or `None` when display shows all of them.
    pub fn last(self, shape: &[usize]) -> Option<usize> {
        let columns = (self.columns != usize::MAX).then_some(self.columns);
        self.edges(shape).map_or(columns, |e| Some(columns.map_or(e, |c| c.min(e))))
    }
    /// The text of `n` with each float rounded to `prec` significant digits.
    pub fn number(self, n: &Number) -> String { n.rounded(self.prec).to_string() }
}

/// The positions display shows along an axis of `len` items: all of them, or the first and last `edges` with `None` for
/// the gap between.
pub(crate) fn positions(len: usize, edges: Option<usize>) -> impl Iterator<Item = Option<usize>> + Clone {
    let gap = edges.filter(|&e| len > 2 * e);
    let (head, tail) = gap.map_or((len, len), |e| (e, len - e));
    (0..head).map(Some).chain(gap.map(|_| None)).chain((tail..len).map(Some))
}

/// The index of each position display shows on the axes of `shape`, in order, with `None` for a gap. The positions under a
/// gap collapse into it. Each comes with the axis whose index changed to reach it, or `None` for the first.
pub(crate) fn frames(shape: &[usize], edges: Option<usize>) -> Vec<(Option<usize>, Option<Vec<usize>>)> {
    let mut frames = vec![(None, Some(vec![]))];
    for (axis, &len) in shape.iter().enumerate() {
        frames = frames
            .into_iter()
            .flat_map(|(changed, index)| match index {
                None => vec![(changed, None)],
                Some(index) => positions(len, edges)
                    .enumerate()
                    .map(|(i, p)| (if i == 0 { changed } else { Some(axis) }, p.map(|p| [&index[..], &[p]].concat())))
                    .collect(),
            })
            .collect();
    }
    frames
}

/// A position display shows: an item's ravel offset, or a marker for elided items.
pub(crate) enum Spot { At(usize), Gap(char) }

/// One row display shows of an array: the blank lines before it, the index of its row, or `None` in a gap, and its spots.
pub(crate) type Row = (usize, Option<Vec<usize>>, Vec<Spot>);

/// The rows display shows of an array of `shape`, keeping `edges` positions at each end of a long axis, and `last` on the last axis.
/// An array of rank 0 is one row of one item. A blank line comes before a row for each axis before the last two whose index
/// changes there. A row in a gap shows `⋮` in each column, and `⋱` in the columns' own gap.
pub(crate) fn rows(shape: &[usize], edges: Option<usize>, last: Option<usize>) -> Vec<Row> {
    let (columns, leading) = shape.split_last().map_or((1, &[][..]), |(&columns, leading)| (columns, leading));
    frames(leading, edges)
        .into_iter()
        .map(|(changed, index)| {
            let breaks = changed.map_or(0, |axis| shape.len() - 2 - axis);
            let start = index.as_ref().map(|index| index.iter().zip(leading).fold(0, |n, (&i, &len)| n * len + i) * columns);
            let spots = positions(columns, last)
                .map(|j| match (start, j) {
                    (Some(start), Some(j)) => Spot::At(start + j),
                    (Some(_), None) => Spot::Gap('…'),
                    (None, Some(_)) => Spot::Gap('⋮'),
                    (None, None) => Spot::Gap('⋱'),
                })
                .collect();
            (breaks, index, spots)
        })
        .collect()
}

/// Display settings for a session. `•prefs` reads and changes them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Settings {
    pub boxed: bool,
    pub trees: bool,
    pub functions: bool,
    pub dissect: bool,
    pub elide: Elide,
}

const KEYS: [&str; 8] = ["box", "trees", "fns", "dissect", "limit", "edges", "prec", "width"];

impl Settings {
    /// The REPL's settings: boxed, with lines elided to the terminal's width.
    pub fn interactive() -> Self {
        let elide = Elide { width: crate::system::terminal_size().map(|(_, columns)| columns).unwrap_or(usize::MAX), ..Elide::default() };
        Self { boxed: true, trees: true, functions: true, elide, ..Self::default() }
    }
    /// Display text for `a`, boxed unless boxing is off, or `inside` a function while boxing there is off. A line wider than `width`
    /// elides columns, keeping as many at each end of the last axes as fit.
    pub fn array(&self, a: &Value, inside: bool) -> String {
        let text = |columns| {
            let elide = Elide { columns, ..self.elide };
            if self.boxed && (!inside || self.functions) { array(a, elide).text() } else { plain(a, elide) }
        };
        let fits = |t: &str| t.lines().all(|line| line.width() <= self.elide.width);
        let full = text(usize::MAX);
        if fits(&full) { return full; }
        // Keeping fewer columns never widens a line, so bisection finds the most that fit. Keeping at least one, as many as
        // `width` always gives a line too wide.
        let (mut fit, mut over) = (1, self.elide.width);
        while over - fit > 1 { let mid = (fit + over) / 2; if fits(&text(mid)) { fit = mid } else { over = mid } }
        text(fit)
    }
    pub fn diagram(&self, a: &Value) -> String { array(a, self.elide).text() }
    /// Text for `⎕←`: the display text, with every item.
    pub fn explicit(&self, a: &Value, inside: bool) -> String { Self { elide: Elide::NONE, ..*self }.array(a, inside) }
    /// The settings as a record. An unlimited count is `∞`.
    fn record(&self) -> Value {
        let flag = |b| Value::Number(Number::from_bool(b));
        let count = |n: usize| Value::Number(if n == usize::MAX { Number::from(f64::INFINITY) } else { Number::from_integer(n as i64) });
        let values = vec![
            flag(self.boxed),
            flag(self.trees),
            flag(self.functions),
            flag(self.dissect),
            count(self.elide.limit),
            count(self.elide.edges),
            count(self.elide.prec),
            count(self.elide.width),
        ];
        keyed::record(KEYS.map(Into::into).to_vec(), values).expect("the settings fit in a record")
    }
    /// The settings with the entries of the record `changes` applied.
    fn update(mut self, changes: &Value) -> Result<Self, crate::ErrorKind> {
        let count = |v: &Value| match v.as_number() {
            Some(n) if n.as_float() == Some(f64::INFINITY) => Ok(usize::MAX),
            Some(n) => n.nonnegative_integer().map(crate::array::saturated),
            None => Err(crate::ErrorKind::Domain),
        };
        let positive = |v: &Value| count(v).and_then(|d| if d == 0 { Err(crate::ErrorKind::Domain) } else { Ok(d) });
        for (key, value) in keyed::pairs(changes)? {
            match &*key {
                "box" => self.boxed = value.boolean()?,
                "trees" => self.trees = value.boolean()?,
                "fns" => self.functions = value.boolean()?,
                "dissect" => self.dissect = value.boolean()?,
                "limit" => self.elide.limit = count(&value)?,
                "edges" => self.elide.edges = count(&value)?,
                "prec" => self.elide.prec = positive(&value)?,
                "width" => self.elide.width = positive(&value)?,
                _ => return Err(crate::ErrorKind::Domain),
            }
        }
        Ok(self)
    }
}

/// `•prefs Y` applies the settings in the record `Y` to the session's display, and returns all of them.
pub(crate) fn prefs(session: &mut crate::Session, _: Option<&Value>, right: &Value, span: &crate::Span) -> Result<Value, crate::Error> {
    session.display = session.display.update(right).error_at(span, "•prefs takes a record of display settings")?;
    Ok(session.display.record())
}

/// Lines of text and their width. Display measures text in terminal columns, and `⍕` in characters.
struct Block { lines: Vec<String>, width: usize, measure: fn(&str) -> usize }
impl Block {
    fn new(text: String) -> Self {
        let lines: Vec<_> = if text.contains('\n') { text.split('\n').map(str::to_owned).collect() } else { vec![text] };
        Self::measured(lines, UnicodeWidthStr::width)
    }
    fn measured(lines: Vec<String>, measure: fn(&str) -> usize) -> Self {
        let width = lines.iter().map(|s| measure(s)).max().unwrap_or(0);
        Self { lines, width, measure }
    }
    fn line(&self, row: usize) -> String {
        let s = self.lines.get(row).map_or("", String::as_str);
        format!("{s}{}", " ".repeat(self.width - (self.measure)(s)))
    }
    fn text(self) -> String { self.lines.join("\n") }
    fn labelled(self, name: &str) -> Self {
        let label = format!("{name}:");
        let pad = " ".repeat(label.width());
        let lines = self.lines.iter().enumerate().map(|(i, s)| format!("{}{s}", if i == 0 { &label } else { &pad })).collect();
        Self { lines, width: self.width + label.width(), ..self }
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
        Self { lines, width: width + 2, ..self }
    }
}

/// A cell of a grid: its text, for a number the widths around its decimal point, and whether it holds an array.
pub(crate) struct Cell { block: Block, number: Option<Decimals>, nested: bool }
impl Cell {
    pub fn text(text: String) -> Self { Self { block: Block::new(text), number: None, nested: false } }
    pub fn number(text: String) -> Self { Self { number: Some(Decimals::of(text.chars())), block: Block::new(text), nested: false } }
    /// A cell for `spot`: `item` for an item's offset, and a marker that lines up with the units digits of numbers.
    pub fn spot(spot: &Spot, item: impl FnOnce(usize) -> Self) -> Self { match *spot { Spot::At(i) => item(i), Spot::Gap(c) => Self::number(c.into()) } }
    /// A cell of `⍕`'s layout for `item`, from the rows of its formatted text.
    pub fn formatted(rows: Vec<Vec<char>>, item: &Value) -> Self {
        Self::item(Block::measured(rows.into_iter().map(String::from_iter).collect(), |s| s.chars().count()), item)
    }
    /// A cell holding `block`, the text of `item`.
    fn item(block: Block, item: &Value) -> Self {
        let number = matches!(item, Value::Number(_)).then(|| Decimals::of(block.lines[0].chars()));
        Self { block, number, nested: matches!(item, Value::Array(_)) }
    }
    pub fn height(&self) -> usize { self.block.lines.len() }
}

/// How a grid spaces and aligns its cells. In every style, numbers in a column line up on their decimal points.
pub(crate) enum Style {
    /// The same spacing and alignment in every column. One space separates the cells of a row when `spaced`. Other cells align left,
    /// or right when `right`.
    Uniform { spaced: bool, right: bool },
    /// Spacing and alignment that depend on what each column holds, as `⍕` lays out an array of mixed or nested items. A space
    /// follows a column with numbers, and comes before a column of nothing but numbers. A column with an array has a space after
    /// each cell and before each cell other than a number. Other cells align right in a column with numbers, and left elsewhere.
    PerColumn,
}

/// The width of a grid with `columns` columns laid out in `style`, and the lines of each of its rows, starting with the row's blank
/// lines.
pub(crate) fn grid(columns: usize, rows: Vec<(usize, Vec<Cell>)>, style: Style) -> (usize, Vec<Vec<String>>) {
    let per_column = matches!(style, Style::PerColumn);
    let mut nested = vec![false; columns];
    for (x, cell) in rows.iter().flat_map(|(_, cells)| cells.iter().enumerate()) { nested[x] |= per_column && cell.nested; }
    // A cell other than a number in a column with an array starts with a space.
    let lead = |x: usize, cell: &Cell| usize::from(nested[x] && cell.number.is_none());
    let (mut widths, mut decimals) = (vec![1; columns], vec![Decimals::default(); columns]);
    for (x, cell) in rows.iter().flat_map(|(_, cells)| cells.iter().enumerate()) {
        widths[x] = widths[x].max(lead(x, cell) + cell.block.width);
        if let Some(n) = cell.number { decimals[x].fit(n); }
    }
    for (w, d) in widths.iter_mut().zip(&decimals) { *w = (*w).max(d.width()); }
    let numeric: Vec<_> = decimals.iter().map(|d| d.width() > 0).collect();
    let (separated, right): (Vec<_>, Vec<_>) = (0..columns)
        .map(|x| match style {
            Style::Uniform { spaced, right } => (x > 0 && spaced, right),
            Style::PerColumn => (x > 0 && ((numeric[x - 1] && !nested[x - 1]) || (numeric[x] && widths[x] == decimals[x].width())), numeric[x]),
        })
        .unzip();
    let width = widths.iter().sum::<usize>() + separated.iter().chain(&nested).filter(|&&b| b).count();
    let lines = rows
        .into_iter()
        .map(|(breaks, cells)| {
            let mut lines = vec![String::new(); breaks];
            for y in 0..cells.iter().map(Cell::height).max().unwrap_or(0).max(1) {
                let mut line = String::new();
                for (x, cell) in cells.iter().enumerate() {
                    if separated[x] { line.push(' '); }
                    let extra = widths[x] - lead(x, cell) - cell.block.width;
                    let (before, after) = match cell.number {
                        Some(n) => {
                            let (before, after) = decimals[x].padding(n);
                            (before + widths[x] - decimals[x].width(), after)
                        }
                        None if right[x] => (extra, 0),
                        None => (0, extra),
                    };
                    line.extend(std::iter::repeat_n(' ', before + lead(x, cell)));
                    line.push_str(&cell.block.line(y));
                    line.extend(std::iter::repeat_n(' ', after + usize::from(nested[x])));
                }
                lines.push(line);
            }
            lines
        })
        .collect();
    (width, lines)
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

/// A boxed diagram of `a`: its items in a frame that marks its shape and storage, with nested arrays boxed in turn.
fn array(a: &Value, el: Elide) -> Block {
    if a.is_unit() || (a.has_keys() && a.shape() == [0]) { return Block::new(plain(a, el)); }
    if a.has_keys() && a.shape().len() > 1 { return Block::new(plain(a, el)).framed(a.shape(), marker(a), false); }
    // An empty axis shows its prototype.
    let shape: Vec<_> = a.shape().iter().map(|&d| d.max(1)).collect();
    let nested = !a.is_simple();
    let kind = if nested { '∊' } else { marker(a) };
    let mut chars = true;
    let mut grid_rows: Vec<(usize, Vec<Cell>)> = Vec::new();
    for (breaks, _, spots) in rows(&shape, el.edges(&shape), el.last(&shape)) {
        let cells = spots
            .iter()
            .map(|spot| {
                Cell::spot(spot, |i| {
                    let item = if a.is_empty() { a.prototype() } else { a.at(i) };
                    chars &= matches!(item, Value::Character(_));
                    let block = match &item {
                        // An `ₓ` box already says its numbers are exact.
                        Value::Number(n) if kind == 'ₓ' => Block::new(format!("{n:#}")),
                        Value::Number(n) => Block::new(el.number(n)),
                        Value::Character(c) => Block::new(if c.is_control() { c.escape_default().to_string() } else { c.to_string() }),
                        Value::Array(_) => array(&item, el),
                        Value::Function(f) => Block::new(f.to_string()),
                        Value::Operator(op) => Block::new(op.to_string()),
                    };
                    let block = match a.keys(0).and_then(|keys| keys.names().get(i)?.clone()) { Some(key) => block.labelled(&key), None => block };
                    Cell::item(block, &item)
                })
            })
            .collect();
        grid_rows.push((breaks, cells));
    }
    let columns = grid_rows[0].1.len();
    let (_, lines) = grid(columns, grid_rows, Style::Uniform { spaced: !(chars && !a.has_keys()), right: false });
    Block::measured(lines.concat(), UnicodeWidthStr::width).framed(a.shape(), kind, nested)
}

/// Display text for `a` without frames: a vector or unit as source text, a string as its characters, and higher ranks as rows.
pub(crate) fn plain(a: &Value, el: Elide) -> String {
    match a {
        Value::Number(n) => return el.number(n),
        Value::Function(f) => return f.to_string(),
        Value::Character(_) | Value::Operator(_) => return a.literal(),
        Value::Array(_) => (),
    }
    if a.has_keys() && a.shape().len() > 1 { return labelled(a, el); }
    let edges = el.edges(a.shape());
    if a.shape().len() <= 1 {
        let string = a.shape().len() == 1
            && !a.has_keys()
            && a.axis_names().iter().all(Option::is_none)
            && !a.is_empty()
            && a.elements().all(|e| matches!(e, Value::Character(_)));
        return if string { a.elided_text(el.last(a.shape())) } else if a.is_strand() { a.shown_items(el.last(a.shape()), |e| e.source(el)).join(" ") } else { a.source(el) };
    }
    if a.is_empty() { return a.literal(); }
    let mut spaced = false;
    let table: Vec<(usize, Vec<Cell>)> = rows(a.shape(), edges, el.last(a.shape()))
        .into_iter()
        .map(|(breaks, _, spots)| {
            let cells = spots
                .iter()
                .map(|spot| {
                    Cell::spot(spot, |i| match a.at(i) {
                        Value::Character(c) => Cell::text(c.into()),
                        item => {
                            spaced = true;
                            match item {
                                Value::Number(n) => Cell::number(el.number(&n)),
                                Value::Function(f) => Cell::text(f.to_string()),
                                Value::Operator(op) => Cell::text(op.to_string()),
                                item => Cell::text(item.item(el)),
                            }
                        }
                    })
                })
                .collect();
            (breaks, cells)
        })
        .collect();
    let columns = table[0].1.len();
    grid(columns, table, Style::Uniform { spaced, right: true }).1.concat().join("\n")
}

/// Display text for an array of rank 2 or more with keys: a table for each matrix, with keys as labels. Each table of a
/// higher rank follows its index and `⌷`.
fn labelled(a: &Value, el: Elide) -> String {
    let rank = a.shape().len();
    let edges = el.edges(a.shape());
    let label = |axis: usize, i: usize| a.keys(axis).and_then(|k| k.names()[i].clone());
    let (rows_keyed, columns_keyed) = (a.keys(rank - 2).is_some(), a.keys(rank - 1).is_some());
    let table = a.shape()[rank - 2] * a.shape()[rank - 1];
    let pages: Vec<_> = frames(&a.shape()[..rank - 2], edges)
        .into_iter()
        .map(|(_, index)| {
            let Some(index) = index else { return "⋮".to_string() };
            let start = index.iter().zip(a.shape()).fold(0, |n, (&i, &len)| n * len + i) * table;
            let mut grid_rows: Vec<(usize, Vec<Cell>)> = Vec::new();
            if columns_keyed {
                let header = positions(a.shape()[rank - 1], el.last(a.shape()))
                    .map(|j| Cell::text(j.map_or_else(|| "…".into(), |j| label(rank - 1, j).map_or_else(|| j.to_string(), |k| k.to_string()))));
                grid_rows.push((0, rows_keyed.then(|| Cell::text(String::new())).into_iter().chain(header).collect()));
            }
            for (_, row, spots) in rows(&a.shape()[rank - 2..], edges, el.last(a.shape())) {
                let name = row.map_or_else(|| "⋮".into(), |row| label(rank - 2, row[0]).map_or_else(|| row[0].to_string(), |k| k.to_string()));
                let cells = spots.iter().map(|spot| Cell::spot(spot, |i| Cell::text(plain(&a.at(start + i), el))));
                grid_rows.push((0, rows_keyed.then(|| Cell::text(name)).into_iter().chain(cells).collect()));
            }
            let columns = grid_rows[0].1.len();
            let text = grid(columns, grid_rows, Style::Uniform { spaced: true, right: true }).1.concat().join("\n");
            if rank == 2 { return text; }
            let coordinates: Vec<_> =
                index.iter().enumerate().map(|(axis, &i)| label(axis, i).map_or_else(|| i.to_string(), |k| crate::array::quoted(&k))).collect();
            format!("{}⌷\n{text}", coordinates.join(" "))
        })
        .collect();
    pages.join("\n\n")
}

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

/// Convert a keyed vector from MIME types to text, or to bytes for binary types such as `image/png`, into the bundle sent to
/// frontends. A value whose prototype is a number holds bytes.
pub(crate) fn bundle(value: &Value) -> Result<crate::MimeBundle, crate::ErrorKind> {
    crate::keyed::pairs(value)?
        .into_iter()
        .map(|(name, value)| {
            if !name.contains('/') || name.contains(char::is_whitespace) { return Err(crate::ErrorKind::Domain); }
            let data = if value.prototype().as_number().is_some() { crate::data::byte_items(&value).map(crate::MimeData::Bytes) } else { keyed::name(&value).map(|text| crate::MimeData::Text(text.to_string())) };
            Ok((name.to_string(), data.ok_or(crate::ErrorKind::Domain)?))
        })
        .collect()
}

/// A native renderer, called with the value to display.
pub(crate) fn renderer(name: &'static str, render: crate::system::Native) -> crate::Function {
    crate::system::native(name, crate::system::Call::Value(render), crate::system::Valence::Monadic)
}

/// A MIME bundle holding `data` as the type `kind`.
pub(crate) fn mime(kind: &str, data: Value) -> Result<Value, crate::ErrorKind> { crate::keyed::vector(vec![kind.into()], vec![data]) }
