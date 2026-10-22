use rustymath::{buffer::FloatWidth, error::Kind};
use std::borrow::Cow;
use crate::{keyed, settings, symbols::Notation, transform::Plain, Described, Number, Value, Error};
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

/// How much of a large array or a long float display shows. An array of more than `limit` items shows the first and last `edges` positions of each axis longer than twice `edges`, and a marker for the positions between. A float shows `prec` significant digits. `width` is the widest line display shows. `columns`, at most `edges` when that applies, is how many positions display keeps at each end of a last axis to fit `width`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Elide {
    pub limit: usize,
    pub edges: usize,
    pub prec: usize,
    pub width: usize,
    pub columns: usize,
    /// The float width that numbers show without a marker: the session's default width. `None` marks every width, as `•literal⁻¹` does.
    pub plain: Option<FloatWidth>,
    /// Whether numbers show their kind: `ₓ` after an exact integer, and the marker of a float width that isn't `plain`. `⍬⍕` and `''⍕` leave both out.
    pub marks: bool,
    pub notation: Notation,
    /// How text shows hidden names, which start with `_`. Source text shows them in full. Box mode shortens them. Plain display leaves hidden axis names out and shortens hidden keys.
    pub hidden: Hidden,
}
/// How text shows a hidden name, which starts with `_`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Hidden { Full, Short, Omit }
impl Default for Elide { fn default() -> Self { Self { limit: 1000, edges: 3, prec: 10, ..Self::NONE } } }
impl Elide {
    /// Shows every item and every digit, as source text and `⍕` do, with 64-bit floats plain.
    pub const NONE: Self =
        Self { limit: usize::MAX, edges: 0, prec: usize::MAX, width: usize::MAX, columns: usize::MAX, plain: Some(FloatWidth::F64), marks: true, notation: Notation::Apl, hidden: Hidden::Full };
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
    /// The text of `n` with each float rounded to `prec` significant digits, and the marker of a float width that isn't `plain`. Without `marks`, it is `unmarked`'s text.
    pub fn number(self, n: &Number) -> String {
        if !self.marks { return self.unmarked(n); }
        format!("{}{}", n.rounded(self.prec), self.marker(n).map_or(String::new(), String::from))
    }
    /// The text of `n` rounded as `number` rounds it, without a marker, for an array that writes one marker for all its numbers.
    pub fn unmarked(self, n: &Number) -> String { format!("{:#}", n.rounded(self.prec)) }
    /// The marker of a float of `width`, when markers show and the width isn't `plain`.
    pub fn width_marker(self, width: FloatWidth) -> Option<char> { (self.marks && Some(width) != self.plain).then(|| crate::number::width_marker(width)) }
    /// The width marker that `n` writes: that of its float width, when it isn't `plain`.
    pub fn marker(self, n: &Number) -> Option<char> { n.float_width().and_then(|w| self.width_marker(w)) }
    /// `name` as text shows it: `None` for a hidden name that `hidden` leaves out, otherwise as `shown` gives it.
    pub fn name(self, name: &Option<std::sync::Arc<str>>) -> Option<Cow<'_, str>> {
        name.as_deref().filter(|n| self.hidden != Hidden::Omit || !rustymath::layout::hidden(n)).map(|n| self.shown(n))
    }
    /// `text` as display shows it: a hidden name cut after its last `system::FRESH`, as in `_b·…`, unless `hidden` is `Full`.
    pub fn shown(self, text: &str) -> Cow<'_, str> {
        match text.rfind(crate::system::FRESH) {
            Some(i) if self.hidden != Hidden::Full && rustymath::layout::hidden(text) => format!("{}…", &text[..i + crate::system::FRESH.len_utf8()]).into(),
            _ => text.into(),
        }
    }
}

/// The positions display shows along an axis of `len` items: all of them, or the first and last `edges` with `None` for the gap between.
pub(crate) fn positions(len: usize, edges: Option<usize>) -> impl Iterator<Item = Option<usize>> + Clone {
    let gap = edges.filter(|&e| len > 2 * e);
    let (head, tail) = gap.map_or((len, len), |e| (e, len - e));
    (0..head).map(Some).chain(gap.map(|_| None)).chain((tail..len).map(Some))
}

/// The index of each position display shows on the axes of `shape`, in order, with `None` for a gap. The positions under a gap collapse into it. Each comes with the axis whose index changed to reach it, or `None` for the first.
pub(crate) fn frames(shape: &[usize], edges: Option<usize>) -> Vec<(Option<usize>, Option<Vec<usize>>)> {
    let mut frames = vec![(None, Some(vec![]))];
    for (axis, &len) in shape.iter().enumerate() {
        frames = frames.into_iter().flat_map(|(changed, index)| match index {
            None => vec![(changed, None)], Some(index) => positions(len, edges).enumerate()
                .map(|(i, p)| (if i == 0 { changed } else { Some(axis) }, p.map(|p| [&index[..], &[p]].concat()))).collect()
        }).collect();
    }
    frames
}

/// A position display shows: an item's ravel offset, or a marker for elided items.
pub(crate) enum Spot { At(usize), Gap(char) }

/// One row display shows of an array: the blank lines before it, the index of its row, or `None` in a gap, and its spots.
pub(crate) type Row = (usize, Option<Vec<usize>>, Vec<Spot>);

/// The rows display shows of an array of `shape`, keeping `edges` positions at each end of a long axis, and `last` on the last axis. An array of rank 0 is one row of one item. A blank line comes before a row for each axis before the last two whose index changes there. A row in a gap shows `⋮` in each column, and `⋱` in the columns' own gap.
pub(crate) fn rows(shape: &[usize], edges: Option<usize>, last: Option<usize>) -> Vec<Row> {
    let (columns, leading) = shape.split_last().map_or((1, &[][..]), |(&columns, leading)| (columns, leading));
    frames(leading, edges).into_iter().map(|(changed, index)| {
        let breaks = changed.map_or(0, |axis| shape.len() - 2 - axis);
        let start = index.as_ref().map(|index| index.iter().zip(leading).fold(0, |n, (&i, &len)| n * len + i) * columns);
        let spots = positions(columns, last).map(|j| match (start, j) {
            (Some(start), Some(j)) => Spot::At(start + j), (Some(_), None) => Spot::Gap('…'), (None, Some(_)) => Spot::Gap('⋮'),
            (None, None) => Spot::Gap('⋱')
        }).collect();
        (breaks, index, spots)
    }).collect()
}

/// The display settings, which `•prefs` reads and changes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Settings { pub boxed: bool, pub trees: bool, pub functions: bool, pub dissect: bool, pub elide: Elide }

impl Settings {
    /// The REPL's settings: boxed, with lines elided to the terminal's width.
    pub fn interactive() -> Self {
        let elide = Elide { width: crate::system::terminal_size().map(|(_, columns)| columns).unwrap_or(usize::MAX), ..Elide::default() };
        Self { boxed: true, trees: true, functions: true, elide, ..Self::default() }
    }
    /// Display text for `a`, boxed where `boxes` says, unless boxing is off, or `inside` a function while boxing there is off. A line wider than `width` elides columns, keeping as many at each end of the last axes as fit.
    pub fn array(&self, a: &Value, inside: bool) -> String {
        let text = |columns| {
            let elide = Elide { columns, hidden: if self.boxed { Hidden::Short } else { Hidden::Omit }, ..self.elide };
            if self.boxed && (!inside || self.functions) && boxes(a, elide) { array(a, elide).text() } else { plain(a, elide) }
        };
        let fits = |t: &str| t.lines().all(|line| line.width() <= self.elide.width);
        let full = text(usize::MAX);
        if fits(&full) { return full; }
        // Keeping fewer columns never widens a line, so bisection finds the most that fit. Keeping at least one, as many as `width` always gives a line too wide.
        let (mut fit, mut over) = (1, self.elide.width);
        while over - fit > 1 { let mid = (fit + over) / 2; if fits(&text(mid)) { fit = mid } else { over = mid } }
        text(fit)
    }
    pub fn diagram(&self, a: &Value) -> String { array(a, self.elide).text() }
    /// Text for `⎕←`: the display text, with every item, and floats rounded to `prec` as display rounds them and with its float width plain.
    pub fn explicit(&self, a: &Value, inside: bool) -> String {
        Self { elide: Elide { prec: self.elide.prec, plain: self.elide.plain, notation: self.elide.notation, ..Elide::NONE }, ..*self }.array(a, inside)
    }
    /// Each setting's key and value. An unlimited count is `∞`, a float of `width`.
    fn entries(&self, width: FloatWidth) -> [(&'static str, Value); 9] {
        let flag = |b| Value::Number(Number::from_bool(b));
        let count = |n: usize| Value::Number(if n == usize::MAX { Number::float(f64::INFINITY, width) } else { Number::from_integer(n as i64) });
        [
            ("box", flag(self.boxed)), ("trees", flag(self.trees)), ("fns", flag(self.functions)), ("dissect", flag(self.dissect)),
            ("limit", count(self.elide.limit)), ("edges", count(self.elide.edges)), ("prec", count(self.elide.prec)), ("width", count(self.elide.width)), (
                "notation", keyed::text(match self.elide.notation {
                    Notation::Apl => "apl", Notation::Simplified => "simplified", Notation::Traditional => "traditional"
                })),
        ]
    }
    /// Sets the setting `key` to `value`. `Ok(false)` when no display setting has that key.
    fn set(&mut self, key: &str, value: &Value) -> Result<bool, crate::ErrorKind> {
        let count = |v: &Value| match v.as_number() {
            Some(n) if n.as_float() == Some(f64::INFINITY) => Ok(usize::MAX), Some(n) => n.nonnegative_integer().map(crate::array::saturated),
            None => Err(Kind::Domain.into())
        };
        let positive = |v: &Value| count(v).and_then(|d| if d == 0 { Err(Kind::Domain.into()) } else { Ok(d) });
        match key {
            "box" => self.boxed = value.boolean()?,
            "trees" => self.trees = value.boolean()?,
            "fns" => self.functions = value.boolean()?,
            "dissect" => self.dissect = value.boolean()?,
            "limit" => self.elide.limit = count(value)?,
            "edges" => self.elide.edges = count(value)?,
            "prec" => self.elide.prec = positive(value)?,
            "width" => self.elide.width = positive(value)?,
            "notation" => {
                self.elide.notation = match keyed::name(value).as_deref() {
                    Some("apl") => Notation::Apl, Some("simplified") => Notation::Simplified, Some("traditional") => Notation::Traditional,
                    _ => return Err(Kind::Domain.into())
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

/// `•prefs Y` applies the settings in the record `Y`, and returns all of them. Settings hold for the whole process (`settings`). The result is shy when `Y` sets something, as an assignment's is, so `•prefs ⍬` shows the settings. Besides the display settings, `tolerance` is the comparison tolerance of 64-bit floats, a number from 0 to `1E¯14`. Narrower floats compare exactly. `float` is the width of new floats in bits, 16, 32 or 64. Code parsed after a change reads its literals at the new width. `kinds` is `"dynamic"`, the default, or `"static"`, under which every result's kind follows from the operation, its arguments' kinds and these settings, never from values. `int` is the width of new integers under static kinds, 32 or 64. `fusion` makes each call that computes a pending argument report it. `threads` is the number of threads that large calls split across, from then on. 0 gives the default. 1 runs them on the calling thread alone. `seed`, a nonnegative integer, seeds the session's random numbers, which `¿` and a distribution's `sample` draw. The returned settings leave it out, because the stream moves on as it draws. `batch` makes Each, Rank, stencils and outer products first try one call of their operand on batches of all their cells (`batch::tried`). Where that call can't give every cell's result, they call the operand once for each cell. The results are the same either way. `batch` starts on.
pub(crate) fn prefs(session: &crate::Session, right: &Value) -> Result<(Value, bool), crate::Error> {
    let (mut display, mut numeric, mut threads, mut seed) = (settings::display(), settings::numeric(), None, None);
    let (mut hanzi_aliases, mut batch, mut fusion) = (settings::hanzi_aliases(), settings::batch(), settings::reports_fusion());
    let pairs = keyed::pairs(right).described("•prefs takes a record of settings")?;
    let shy = !pairs.is_empty();
    if shy { crate::transform::acts()?; }
    for (key, value) in pairs {
        if numeric.set(&key, &value).map_err(|message| Error::domain(message))? { continue; }
        match &*key {
            "hanzi_aliases" => {
                if !session.prefacing.get() { return Err(Error::domain("hanzi_aliases needs a leading •prefs declaration on a literal record")); }
                hanzi_aliases = value.boolean().described("hanzi_aliases must be Boolean")?;
            }
            "threads" => {
                let n = value.as_number().and_then(|n| n.integer().ok()).and_then(|n| usize::try_from(n).ok());
                threads = Some(n.ok_or(Kind::Domain).described("threads must be a nonnegative integer")?);
            }
            "seed" => {
                let n = value.as_number().and_then(|n| n.nonnegative_integer::<u64>().ok());
                seed = Some(n.ok_or(Kind::Domain).described("seed must be a nonnegative integer")?);
            }
            "batch" => batch = value.boolean().described("batch must be Boolean")?,
            "fusion" => fusion = value.boolean().described("fusion must be Boolean")?,
            key => {
                if !display.set(key, &value).described("invalid display setting")? { return Err(Error::domain("unknown setting")); }
            }
        }
    }
    settings::set_display(display);
    settings::set_numeric(numeric);
    settings::set_hanzi_aliases(hanzi_aliases);
    settings::set_batch(batch);
    settings::set_reports_fusion(fusion);
    if let Some(n) = threads { rustymath::parallel::set_threads(n); }
    if let Some(seed) = seed { *session.generator.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = crate::distribution::Stream::seeded(seed); }
    let flag = |b| Value::Number(Number::from_bool(b));
    let entries = display.entries(numeric.width).into_iter().chain(numeric.entries()).chain([
        ("fusion", flag(fusion)), ("threads", Value::integer(rustymath::parallel::thread_setting() as i64)), ("hanzi_aliases", flag(hanzi_aliases)), ("batch", flag(batch)),
    ]);
    let (keys, values): (Vec<std::sync::Arc<str>>, Vec<Value>) = entries.map(|(k, v)| (k.into(), v)).unzip();
    Ok((keyed::vector(keys, values).described("invalid settings record")?, shy))
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
        let number = matches!(item.displayed(), Plain::Number(_)).then(|| Decimals::of(block.lines[0].chars()));
        Self { block, number, nested: matches!(item, Value::Array(_)) }
    }
    pub fn height(&self) -> usize { self.block.lines.len() }
}

/// How a grid spaces and aligns its cells. In every style, numbers in a column line up on their decimal points.
pub(crate) enum Style {
    /// The same spacing and alignment in every column. One space separates the cells of a row when `spaced`. Other cells align left, or right when `right`.
    Uniform { spaced: bool, right: bool },
    /// Spacing and alignment that depend on what each column holds, as `⍕` lays out an array of mixed or nested items. A space follows a column with numbers, and comes before a column of nothing but numbers. A column with an array has a space after each cell and before each cell other than a number. Other cells align right in a column with numbers, and left elsewhere.
    PerColumn,
}

/// The width of a grid with `columns` columns laid out in `style`, and the lines of each of its rows, starting with the row's blank lines.
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
    let (separated, right): (Vec<_>, Vec<_>) = (0..columns).map(|x| match style {
        Style::Uniform { spaced, right } => (x > 0 && spaced, right),
        Style::PerColumn => (x > 0 && ((numeric[x - 1] && !nested[x - 1]) || (numeric[x] && widths[x] == decimals[x].width())), numeric[x]),
    }).unzip();
    let width = widths.iter().sum::<usize>() + separated.iter().chain(&nested).filter(|&&b| b).count();
    let lines = rows.into_iter().map(|(breaks, cells)| {
        let mut lines = vec![String::new(); breaks];
        for y in 0..cells.iter().map(Cell::height).max().unwrap_or(0).max(1) {
            let mut line = String::new();
            for (x, cell) in cells.iter().enumerate() {
                if separated[x] { line.push(' '); }
                let extra = widths[x] - lead(x, cell) - cell.block.width;
                let (before, after) = match cell.number {
                    Some(n) => { let (before, after) = decimals[x].padding(n); (before + widths[x] - decimals[x].width(), after) }
                    None if right[x] => (extra, 0), None => (0, extra)
                };
                line.extend(std::iter::repeat_n(' ', before + lead(x, cell)));
                line.push_str(&cell.block.line(y));
                line.extend(std::iter::repeat_n(' ', after + usize::from(nested[x])));
            }
            lines.push(line);
        }
        lines
    }).collect();
    (width, lines)
}

/// The bottom-edge marker for an array's storage. Floats of a width that `el` doesn't leave plain show that width's marker.
fn marker(a: &Value, el: Elide) -> char {
    use rustymath::buffer::DType;
    match a.shown_dtype() {
        Some(DType::Boolean) => '$', Some(DType::Integer(_)) => 'ₓ', Some(DType::Float(w)) => el.width_marker(w).unwrap_or('~'),
        Some(DType::Complex(_)) => 'j', Some(DType::Character) => '─', None => '+'
    }
}

/// A boxed diagram of `a`: its items in a frame that marks its shape and storage, with nested arrays boxed in turn.
fn array(a: &Value, el: Elide) -> Block {
    if a.is_unit() || (a.has_keys() && a.shape() == [0]) { return Block::new(plain(a, el)); }
    if a.has_keys() && a.shape().len() > 1 {
        // The table inside leaves out the kind that the frame's corner shows.
        let inner = if a.mark(el).is_some() { Elide { marks: false, ..el } } else { el };
        return Block::new(plain(a, inner)).framed(a.shape(), marker(a, el), false);
    }
    // An empty axis shows its prototype.
    let shape: Vec<_> = a.shape().iter().map(|&d| d.max(1)).collect();
    let nested = !a.is_simple();
    let kind = if nested { '∊' } else { marker(a, el) };
    let mut chars = true;
    let mut grid_rows: Vec<(usize, Vec<Cell>)> = Vec::new();
    for (breaks, _, spots) in rows(&shape, el.edges(&shape), el.last(&shape)) {
        let cells = spots.iter().map(|spot| {
                Cell::spot(spot, |i| {
                    let item = if a.is_empty() { a.prototype() } else { a.at(i) };
                    chars &= matches!(item, Value::Character(_));
                    let block = match item.displayed() {
                        // An `ₓ` box already says its numbers are exact, and a width's marker gives their width.
                        Plain::Number(n) if Some(kind) == a.mark(el) => Block::new(el.unmarked(n)),
                        Plain::Number(n) => Block::new(el.number(n)),
                        Plain::Character(c) => Block::new(if c.is_control() { c.escape_default().to_string() } else { c.to_string() }),
                        Plain::Array(item) => array(item, el),
                        Plain::Function(_) | Plain::Operator(_) => Block::new(plain(&item, el)),
                    };
                    let block = match a.shown_key(0, i, el) { Some(key) => block.labelled(&key), None => block };
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

/// Whether plain display shows `a` as its raw text, as it shows a string: an unkeyed nonempty vector of characters that shows no axis name. The text alone doesn't show that its items are characters.
fn raw_text(a: &Value, el: Elide) -> bool {
    a.shape().len() == 1 && !a.has_keys() && !a.shows_names(el) && !a.is_empty() && a.elements().all(|e| matches!(e, Value::Character(_)))
}

/// Whether box mode draws `a` in a box. Every simple vector but a string keeps its plain form, which shows its kind, keys and axis name. A string's plain form is its raw text, and it keeps its box.
fn boxes(a: &Value, el: Elide) -> bool { a.shape().len() != 1 || !a.is_simple() || raw_text(a, el) }

/// Display text for `a` without frames: a vector or unit as source text, a string as its characters, and higher ranks as rows.
pub(crate) fn plain(a: &Value, el: Elide) -> String {
    match a.displayed() {
        Plain::Number(n) => return el.number(n), Plain::Function(f) => return format!("⟨{}⟩", f.text(el.notation, &mut 1000)),
        Plain::Operator(op) => return op.text(el.notation), Plain::Character(_) => return a.literal(), Plain::Array(_) => ()
    }
    if a.has_keys() && a.shape().len() > 1 { return labelled(a, el); }
    if a.shape().len() <= 1 {
        return if raw_text(a, el) { a.elided_text(el.last(a.shape())) }
        else if a.is_strand(el) && a.mark(el).is_none() { a.shown_items(el.last(a.shape()), |e| e.source(el)).join(" ") }
        else { a.source(el) };
    }
    if a.is_empty() { return a.literal(); }
    // Numbers that share a marked kind write as their literal laid out in rows, with one mark after its brackets, as a vector does.
    let mark = a.mark(el);
    let (_, rows) = table(a, el, mark.is_none());
    match mark {
        Some(m) => bracketed(rows, &a.shape()[..a.shape().len() - 1], a.shape().len() - 1, m),
        None => rows.into_iter().flat_map(|(_, lines)| lines).collect(),
    }.join("\n")
}

/// The rows of an unkeyed array of rank 2 or more as `plain` lays them out, with the grid's width: each row's index along the leading axes, or `None` in a gap, and its lines. With `breaks`, a row starts with a blank line for each leading axis but the last whose index changes there. Numbers show unmarked where the array's own mark covers them (`Value::mark`).
pub(crate) fn table(a: &Value, el: Elide, breaks: bool) -> (usize, Vec<(Option<Vec<usize>>, Vec<String>)>) {
    let mark = a.mark(el);
    // Numbers space their columns, in an array with no rows too.
    let mut spaced = !matches!(a.prototype(), Value::Character(_));
    let (indices, cells): (Vec<_>, Vec<_>) = rows(a.shape(), el.edges(a.shape()), el.last(a.shape())).into_iter().map(|(blank, index, spots)| {
        let cells: Vec<Cell> = spots.iter().map(|spot| {
            Cell::spot(spot, |i| match a.at(i) {
                Value::Character(c) => Cell::text(c.into()),
                item => {
                    spaced = true;
                    match item.displayed() {
                        Plain::Number(n) if mark.is_some() => Cell::number(el.unmarked(n)),
                        Plain::Number(n) => Cell::number(el.number(n)), Plain::Function(_) | Plain::Operator(_) => Cell::text(plain(&item, el)),
                        _ => Cell::text(item.item(el))
                    }
                }
            })
        }).collect();
        (index, (if breaks { blank } else { 0 }, cells))
    }).unzip();
    // An array with no rows, as `⍕` of an empty matrix gives, takes its columns from its shape.
    let columns = cells.first().map_or_else(|| a.shape().last().copied().unwrap_or(1), |(_, cells)| cells.len());
    let (width, lines) = grid(columns, cells, Style::Uniform { spaced, right: true });
    (width, indices.into_iter().zip(lines).collect())
}

/// The `rows` of a grid as a literal lays them out: before a row, a `[` for each of the last `levels` leading axes whose index starts there, and after it a `]` for each that ends there, with `mark` after the brackets that close all `levels`. A row in a gap gets neither. Each row is one line, as rows of numbers are.
pub(crate) fn bracketed(rows: Vec<(Option<Vec<usize>>, Vec<String>)>, leading: &[usize], levels: usize, mark: char) -> Vec<String> {
    let axes = leading.len() - levels..leading.len();
    rows.into_iter().flat_map(|(index, lines)| {
        let edge = |at: &dyn Fn(usize) -> bool| axes.clone().rev().take_while(|&a| at(a)).count();
        let (opens, closes) = index.map_or((0, 0), |ix| (edge(&|a| ix[a] == 0), edge(&|a| ix[a] + 1 == leading[a])));
        let end = if closes == levels { String::from(mark) } else { String::new() };
        lines.into_iter().map(move |line| format!("{}{}{line}{}{end}", " ".repeat(levels - opens), "[".repeat(opens), "]".repeat(closes)))
    }).collect()
}

/// Display text for an array of rank 2 or more with keys: a table for each matrix, with keys as labels. Items that share a marked kind show the mark once, in the corner. Each table of a higher rank follows its index and `⌷`.
fn labelled(a: &Value, el: Elide) -> String {
    let rank = a.shape().len();
    let edges = el.edges(a.shape());
    let label = |axis: usize, i: usize| a.shown_key(axis, i, el);
    let (rows_keyed, columns_keyed) = (a.keys(rank - 2).is_some(), a.keys(rank - 1).is_some());
    let table = a.shape()[rank - 2] * a.shape()[rank - 1];
    // Items that share a marked kind show the mark once, in each table's corner, and their numbers unmarked. A corner needs a header row and a label column.
    let mark = a.mark(el);
    let items = if mark.is_some() { Elide { marks: false, ..el } } else { el };
    let (header, names) = (columns_keyed || mark.is_some(), rows_keyed || mark.is_some());
    let pages: Vec<_> = frames(&a.shape()[..rank - 2], edges).into_iter().map(|(_, index)| {
        let Some(index) = index else { return "⋮".to_string() };
        let start = index.iter().zip(a.shape()).fold(0, |n, (&i, &len)| n * len + i) * table;
        let mut grid_rows: Vec<(usize, Vec<Cell>)> = Vec::new();
        if header {
            let labels = positions(a.shape()[rank - 1], el.last(a.shape())).map(|j| Cell::text(if !columns_keyed { String::new() } else {
                j.map_or_else(|| "…".into(), |j| label(rank - 1, j).map_or_else(|| j.to_string(), |k| k.to_string()))
            }));
            grid_rows.push((0, names.then(|| Cell::text(mark.map_or_else(String::new, String::from))).into_iter().chain(labels).collect()));
        }
        for (_, row, spots) in rows(&a.shape()[rank - 2..], edges, el.last(a.shape())) {
            let name = if !rows_keyed { String::new() } else { row.map_or_else(|| "⋮".into(), |row| label(rank - 2, row[0]).map_or_else(|| row[0].to_string(), |k| k.to_string())) };
            let cells = spots.iter().map(|spot| Cell::spot(spot, |i| Cell::text(plain(&a.at(start + i), items))));
            grid_rows.push((0, names.then(|| Cell::text(name)).into_iter().chain(cells).collect()));
        }
        let columns = grid_rows[0].1.len();
        let text = grid(columns, grid_rows, Style::Uniform { spaced: true, right: true }).1.concat().join("\n");
        if rank == 2 { return text; }
        let coordinates: Vec<_> =
            index.iter().enumerate().map(|(axis, &i)| label(axis, i).map_or_else(|| i.to_string(), |k| crate::array::quoted(&k))).collect();
        format!("{}⌷\n{text}", coordinates.join(" "))
    }).collect();
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

/// Convert a keyed vector from MIME types to text, or to bytes for binary types such as `image/png`, into the bundle sent to frontends. A value whose prototype is a number holds bytes.
pub(crate) fn bundle(value: &Value) -> Result<crate::MimeBundle, crate::ErrorKind> {
    crate::keyed::pairs(value)?.into_iter().map(|(name, value)| {
        if !name.contains('/') || name.contains(char::is_whitespace) { return Err(Kind::Domain.into()); }
        let data = if value.prototype().as_number().is_some() { crate::data::byte_items(&value).map(crate::MimeData::Bytes) }
        else { keyed::name(&value).map(|text| crate::MimeData::Text(text.to_string())) };
        Ok((name.to_string(), data.ok_or(Kind::Domain)?))
    }).collect()
}

/// A native renderer, called with the value to display, and with the left argument that `before` binds to it, if any.
pub(crate) fn renderer(name: &'static str, render: crate::system::Native) -> crate::Function {
    crate::system::native(name, crate::system::Call::Value(render), crate::system::Valence::Ambivalent)
}

/// A MIME bundle holding `data` as the type `kind`.
pub(crate) fn mime(kind: &str, data: Value) -> Result<Value, crate::ErrorKind> { crate::keyed::vector(vec![kind.into()], vec![data]) }
