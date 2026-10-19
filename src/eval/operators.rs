//! The operators: composition, power, inverse, key, outer and inner products, rank, each, reduce, scan, stencil, `@` and agenda.

use rustymath::error::Kind;
use super::*;

// Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
#[inline(never)]
pub(super) fn composition(op: OperatorKind, operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    use OperatorKind::*;
    if matches!(op, Under) { return under(operands, left, right, cx); }
    if matches!(op, Agenda) { return agenda(operands, left, right, cx); }
    if matches!(op, Stencil) {
        if left.is_some() { return Err(cx.span.error(ErrorKind::Syntax, "stencil is monadic")); }
        let [Operand::Function(f), spec] = operands else { unreachable!() };
        return stencil(f, &computed(spec, None, right, cx)?, right, cx);
    }
    if matches!(op, Power) { return power(operands, left, right, cx); }
    match (&operands[0], &operands[1]) {
        (Operand::Value(a), Operand::Function(f)) if matches!(op, Before) => {
            if left.is_some() { return Err(cx.span.error(ErrorKind::Syntax, "a bound function takes one argument")); }
            f.call(Some(a), right, cx)
        }
        (Operand::Function(f), Operand::Value(a)) if matches!(op, After) => {
            if left.is_some() { return Err(cx.span.error(ErrorKind::Syntax, "a bound function takes one argument")); }
            f.call(Some(right), a, cx)
        }
        (Operand::Function(f), ranks) if matches!(op, Rank) => rank(f, &computed(ranks, left, right, cx)?, left, right, cx),
        (Operand::Function(f), spec) if matches!(op, Axis) => along_axes(f, &computed(spec, left, right, cx)?, left, right, cx),
        (Operand::Function(f), Operand::Function(g)) => match op {
            Valences => { if left.is_some() { g.call(left, right, cx) } else { f.call(None, right, cx) } } Product => inner(f, g, left, right, cx),
            After => { let y = g.call_array(None, right, cx)?; f.call(Some(left.unwrap_or(right)), &y, cx) }
            Atop => { let y = g.call_array(left, right, cx)?; f.call(None, &y, cx) }
            Over => over(f, g, left, right, cx),
            Before => { let x = f.call_array(None, left.unwrap_or(right), cx)?; g.call(Some(&x), right, cx) } _ => unreachable!()
        },
        _ => Err(cx.span.domain_error("invalid operator operands")),
    }
}

/// The case that the unit `index` picks from `len` cases: a whole number, which counts back from the end where it's negative.
pub(super) fn agenda_index(index: &Value, len: usize, span: &Span) -> Result<usize, Error> {
    let n = index.as_number().ok_or_else(|| span.domain_error("agenda index must be numeric"))?;
    crate::primitive::position(&n, len, span)
}

/// `selector⍚cases`. A unit selector picks one case, which runs on the whole arguments. Any other selector picks a case for each cell (`Cells::chosen`).
fn agenda(operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [selector, Operand::Value(fs)] = operands else { unreachable!() };
    let index = computed(selector, left, right, cx)?;
    if !index.is_unit() {
        let cases: Vec<_> = fs.elements().map(|f| match f { Value::Function(f) => f, _ => unreachable!() }).collect();
        return Ok(Bound::from(Cells::new(&index, left, right, cx.span)?.chosen(&index, &cases, left, right, cx)?));
    }
    let Value::Function(f) = fs.at(agenda_index(&index, fs.len(), cx.span)?) else { unreachable!() };
    f.call(left, right, cx)
}

/// An operand that gives an array: the array itself, or a function's result on the arguments, as `⍤`, `⍠`, `⌺` and `⍚` take them.
fn computed(operand: &Operand, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    match operand { Operand::Value(a) => Ok(a.clone()), Operand::Function(g) => g.call_array(left, right, cx) }
}

/// `f@g`: `f` applied to the selection that `g` makes from the argument, then written back in its place. An array `f` replaces the selection. An array `g` lists positions along the leading axis, as `[g]⌷` reads them, and a selection of no items leaves the argument as it is, without calling `f`. `[p]` runs `f` on the cells where `p` of the argument is 1 (`masked`). With positions or `[p]`, a left argument is laid over the right argument (`Laid`), and a unit goes whole to `f`.
fn under(operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [Operand::Function(f), g] = operands else { unreachable!() };
    let positions = match g {
        Operand::Function(g) => return under_function(f, g, left, right, cx),
        Operand::Value(a) => match predicate(a) {
            Some(p) => return Ok(Bound::from(masked(f, &p.call_array(None, right, cx)?, left, right, cx)?)),
            None => a,
        },
    };
    let selection = crate::primitive::at_indices(right, positions, cx)?;
    if selection.targets.len() == 0 { return Ok(Bound::from(right.clone())); }
    let x = match left {
        Some(x) if !x.is_unit() => {
            let x = Laid::new(x, &right.layout().axes(0..1), cx.span)?.aligned(cx)?;
            Some(crate::primitive::at_indices(&x, positions, cx)?.read(&x, cx)?)
        }
        _ => None,
    };
    let values = f.call_array(x.as_ref().or(left), &selection.read(right, cx)?, cx)?;
    Ok(Bound::from(selection.write(right, &values, cx)?))
}

/// The function of `[p]`, a one-item vector that holds a function.
fn predicate(a: &Value) -> Option<Function> { if a.shape() != [1] { return None; } match a.at(0) { Value::Function(p) => Some(p), _ => None } }

/// `f@[p]`, where `p` gives the mask `selector`: `f` runs once on the cells that the mask marks 1 (`Cells::marked`), and its results go back in their places under assignment's rule. Every other cell keeps its value, including one marked NaN, or whose key the mask lacks. A unit mask marks the whole argument or none of it.
fn masked(f: &Function, selector: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    if selector.shape().is_empty() {
        let [k] = crate::primitive::positions(selector, 2, cx)?[..] else { unreachable!("a unit has one position") };
        if k != 1 { return Ok(right.clone()); }
        let every = Selection { frame: ResultFrame::Array(right.layout().clone()), targets: Targets::Offsets((0..right.len()).collect()) };
        return every.write(right, &f.call_array(left, right, cx)?, cx);
    }
    let cells = Cells::new(selector, left, right, cx.span)?;
    let rows = cells.marked(selector, cx)?;
    if rows.is_empty() { return Ok(right.clone()); }
    let selection = cells.framed.selection(&rows, cx.span)?;
    let values = cells.case(f, &rows, left, right, cx)?;
    selection.write(right, &values, cx)
}

/// A case's result, classified once: one cell for each cell that picks the case, or one item that fills them all, as `@` fills a selection.
enum Made { Cells(Value), Fill(Value) }

impl Made {
    /// `values` from a case whose cells have the list frame `rows`. A singleton fills; any other result aligns by key and needs one result per row.
    fn new(values: Value, rows: &crate::array::Layout, span: &Span) -> Result<Self, Error> {
        if values.is_singleton() { return Ok(Self::Fill(values)); }
        let values = crate::keyed::reorder(&values, &rows.all_keys(), true).error_at(span, "case result does not supply selected keys")?;
        if values.shape().first() == rows.shape().first() { return Ok(Self::Cells(values)); }
        Err(span.error(Kind::Length, "a case needs one result for each cell that picks it"))
    }
    fn value(&self) -> &Value { match self { Self::Cells(v) | Self::Fill(v) => v } }
}

/// The cells of `right` along the axes that a selector's axes pair with, for `⍚` with a selector for each cell and for `@[p]`. The selector's axes pair with the argument's by name and then from the front. The selector, and a left argument that isn't a unit, are each laid over the frame as agreement lays them (`lookup`, `Laid`). Cells are read, and written, where they lie (`Framed`): no axis of either argument moves.
struct Cells { framed: Framed, axes: Vec<usize>, lookup: Mapping, x: Option<Laid>, len: usize }

impl Cells {
    fn new(selector: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Self, Error> {
        let axes = rustymath::agreement::Agreement::paired_axes(right.layout(), selector.layout(), true);
        if axes.iter().any(|&a| a >= right.shape().len()) { return Err(span.error(Kind::Rank, "the selector has an axis that the argument lacks")); }
        let framed = Framed::new(right.layout(), &axes, span)?;
        let frame = framed.frame();
        let len = frame.shape().iter().product();
        let lookup = lookup(selector.layout(), &frame, len, "the selector doesn't agree with the argument", span)?;
        let x = left.filter(|x| !x.is_unit()).map(|x| Laid::new(x, &frame, span)).transpose()?;
        Ok(Self { framed, axes, lookup, x, len })
    }

    /// The frame positions that `selector` marks 1, in order. A Boolean mask that lies in frame order goes straight to `⍸`'s kernel. Any other mask decodes as `picked` decodes it.
    fn marked(&self, selector: &Value, cx: &Context<'_>) -> Result<Vec<u32>, Error> {
        if let (Mapping::Linear(1), crate::array::Items::Compact(rustymath::buffer::View::Booleans(mask))) = (&self.lookup, selector.as_items()) {
            let mut rows = Vec::new();
            rustymath::gather::positions(&mut rows, mask, mask.iter().map(|&b| usize::from(b)).sum(), |j| j as u32);
            return Ok(rows);
        }
        Ok(self.picked(selector, 2, cx)?.each().nth(1).unwrap_or_default().to_vec())
    }

    /// The frame positions grouped by the case of `cases` that each picks (`picked`).
    fn picked(&self, selector: &Value, cases: usize, cx: &Context<'_>) -> Result<rustymath::search::Groups, Error> {
        picked(selector, (!matches!(self.lookup, Mapping::Linear(1))).then_some(&self.lookup), self.len, cases, cx)
    }

    /// The left argument's cells at frame positions `rows`, or `None` where the left argument is a unit or absent.
    fn left_cells(&self, rows: &[u32], cx: &Context<'_>) -> Result<Option<Value>, Error> { self.x.as_ref().map(|x| x.cells(rows, cx)).transpose() }

    /// The result of case `f` on the cells of `right` at frame positions `rows`. `f` gets the left argument's cells at the same positions, or `left` whole where it's a unit. A constant case doesn't look at its cells.
    fn case(&self, f: &Function, rows: &[u32], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
        if let Some(a) = f.as_constant() { return Ok(a.clone()); }
        let xs = self.left_cells(rows, cx)?;
        f.call_array(xs.as_ref().or(left), &self.framed.read(right, rows, cx)?, cx)
    }

    /// `⍚` with a selector for each cell. Each case runs once, on the list of all the cells that pick it (`case`), and gives one result for each of them or one item for all of them (`Made`). A case that no cell picks doesn't run. A cell that picks no case gets the fill. Numbers keep their kinds. A case of `⊢` doesn't run: when every other result has the argument's cells, the result starts from the argument, so `⊢`'s cells stay where they are. Otherwise `⊢`'s cells join the other results.
    fn chosen(&self, selector: &Value, cases: &[Function], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
        let span = cx.span;
        if self.len == 0 { return self.no_cells(selector, cases, left, right, cx); }
        let groups = self.picked(selector, cases.len(), cx)?;
        let picked: Vec<&[u32]> = groups.each().collect();
        let keeps = |k: usize| cases[k].as_primitive() == Some(Primitive::Identity(false));
        let mut made = Vec::new();
        for (k, f) in cases.iter().enumerate() {
            let rows = picked[k];
            if rows.is_empty() || keeps(k) { continue }
            let values = self.case(f, rows, left, right, cx)?;
            let frame = listed(&self.framed.frame(), rows, &Default::default(), span)?;
            made.push((k, Made::new(values, &frame, span)?));
        }
        let kept: Vec<usize> = (0..cases.len()).filter(|&k| keeps(k) && !picked[k].is_empty()).collect();
        // The first case that some cell picks gives the fill. A kept case's cells are the argument's.
        let fill = match (kept.first(), made.first()) {
            (Some(&k), first) if first.is_none_or(|(j, _)| k < *j) => right.prototype(),
            (_, Some((_, m))) => m.value().prototype(),
            _ => right.prototype(),
        };
        let cell = self.framed.cells();
        let start = (!kept.is_empty() && made.iter().all(|(_, m)| matches!(m, Made::Fill(_)) || m.value().shape()[1..] == *cell.shape())).then_some(right);
        if start.is_none() {
            for &k in &kept { made.push((k, Made::Cells(self.framed.read(right, picked[k], cx)?))); }
            made.sort_by_key(|&(k, _)| k);
        }
        self.assembled(&picked, picked.get(cases.len()).copied().unwrap_or_default(), &made, fill, start, cx)
    }

    /// The results of the cases as one array, laid out as `placed` gives it. `none` lists the cells that pick no case, which get `fill`. A fill result fills each cell of its case. Results whose cells differ in shape are padded, as Rank pads them, and a fill result spreads across the shape that the others pad to. `start`, where a case of `⊢` keeps its cells and every result has the argument's cells, is the argument, which the result starts from. Numbers keep their kinds.
    fn assembled(&self, picked: &[&[u32]], none: &[u32], made: &[(usize, Made)], fill: Value, start: Option<&Value>, cx: &Context<'_>) -> Result<Value, Error> {
        let span = cx.span;
        let cells = || made.iter().filter_map(|(k, m)| match m { Made::Cells(v) => Some((*k, v)), Made::Fill(_) => None });
        let mut layouts: Vec<_> = cells().map(|(_, v)| v.layout().axes(1..v.shape().len())).collect();
        if start.is_some() || layouts.is_empty() { layouts.push(self.framed.cells()); }
        let cell = crate::array::Layout::assembled(layouts.iter()).error_at(span, "invalid result cells")?;
        let shape = cell.shape();
        if cells().any(|(_, v)| v.shape()[1..] != *shape) {
            let mut items = vec![fill.clone(); self.len];
            for (k, m) in made {
                let rows = picked[*k];
                match m {
                    Made::Cells(v) => {
                        let cells = v.cells(v.shape().len() - 1).and_then(|c| c.framed()).error_at(span, "invalid result")?;
                        for (j, &i) in rows.iter().enumerate() { items[i as usize] = cells.at(j); }
                    }
                    Made::Fill(v) => {
                        let n = shape.iter().product();
                        let mut spread = Gather::items(n);
                        spread.fill(&v.at(0), n);
                        let item = spread.finish(cell.clone(), fill.clone()).error_at(span, "invalid result")?;
                        for &i in rows { items[i as usize] = item.clone(); }
                    }
                }
            }
            // Assembly pads the data; result metadata comes from the case cells, not the scalar no-case placeholders.
            let result = self.framed.frame().assemble(&items, &fill).and_then(|v| v.with_layout(self.framed.frame().concat(&cell)))
                .error_at(span, "invalid result")?;
            let (_, positions) = placed(&self.framed, &self.axes, &cell, span)?;
            if positions.iter().enumerate().all(|(j, &a)| j == a) { return Ok(result); }
            return move_axes(&result, &fronted(&self.axes, self.framed.layout.shape().len()), cx);
        }
        let (layout, positions) = placed(&self.framed, &self.axes, &cell, span)?;
        let target = Framed::new(&layout, &positions, span)?;
        let width: usize = cell.shape().iter().product();
        let mut data = Gather::items(self.len * width);
        // Where every cell picks a case, the case that fills the most cells with one item fills the whole array first.
        let base = if none.is_empty() && start.is_none() { made.iter().filter(|(_, m)| matches!(m, Made::Fill(_))).max_by_key(|(k, _)| picked[*k].len()).map(|(k, _)| *k) } else { None };
        match start {
            Some(argument) => data.walk(argument, 0, &[rustymath::gather::Steps::Stride { start: 0, len: argument.len(), step: 1 }]),
            None => data.fill(&made.iter().find(|(k, _)| Some(*k) == base).map_or_else(|| fill.clone(), |(_, m)| m.value().at(0)), self.len * width),
        }
        let mut write = |rows: &[u32], values: &Value| data.scatter(target.targets(rows).writes().expect("cells' targets are offsets or a walk"), values);
        for (k, m) in made.iter().filter(|(k, _)| Some(*k) != base) { write(picked[*k], m.value()) }
        if start.is_some() && !none.is_empty() { write(none, &fill) }
        data.finish(layout, fill).error_at(span, "invalid result")
    }

    /// `chosen` for a frame with no cells. As Each calls its operand on prototypes, the case that the selector's fill picks runs on the empty list of cells (`on_prototypes`). The result is empty, with that case's cells, laid out as `placed` gives it.
    fn no_cells(&self, selector: &Value, cases: &[Function], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
        let span = cx.span;
        let k = match crate::primitive::positions(&selector.prototype(), cases.len(), cx)?[..] { [k] if k != rustymath::gather::Steps::FILL => k, _ => return Err(span.domain_error("the selector's fill picks no case")) };
        let xs = self.left_cells(&[], cx)?;
        let result = on_prototypes(&cases[k], xs.as_ref().or(left), &self.framed.read(right, &[], cx)?, cx)?.array(span)?;
        let cell = if result.is_singleton() { self.framed.cells() }
            else if result.shape().first() == Some(&0) { result.layout().axes(1..result.shape().len()) } else { Default::default() };
        placed(&self.framed, &self.axes, &cell, span)?.0.collect(Vec::new(), result.prototype()).error_at(span, "invalid result")
    }
}

/// The cells of an array along its frame axes. `layout` and `steps` put the frame's axes first and then a cell's, as `⍉` would, but no item moves. The first `rank` steps reach each cell's start; the rest read its items.
struct Framed { layout: crate::array::Layout, rank: usize, steps: Vec<rustymath::gather::Steps> }

impl Framed {
    fn new(layout: &crate::array::Layout, frame: &[usize], span: &Span) -> Result<Self, Error> {
        let plan = rustymath::structure::transpose(layout, &destination(&fronted(frame, layout.shape().len()))).at(span)?;
        Ok(Self { layout: plan.layout.into(), rank: frame.len(), steps: plan.steps })
    }
    /// The frame's axes, with their names and keys.
    fn frame(&self) -> crate::array::Layout { self.layout.axes(0..self.rank) }
    /// The cells' axes, with their names and keys.
    fn cells(&self) -> crate::array::Layout { self.layout.axes(self.rank..self.layout.shape().len()) }
    /// The starts of `rows`' cells through `lookup`, or fills where it finds none. Interpret the frame's steps once, outside the loop. In frame order, the row positions themselves are the starts; another single stride scales them without coordinate division.
    #[inline]
    fn starts(&self, rows: &[u32], lookup: impl Fn(usize) -> Option<usize>) -> Vec<usize> {
        use rustymath::gather::Steps;
        match self.steps[..self.rank] {
            [Steps::Stride { start: 0, step: 1, .. }] => rows.iter().map(|&j| lookup(j as usize).unwrap_or(Steps::FILL)).collect(),
            [Steps::Stride { start, step, .. }] => rows.iter().map(|&j| lookup(j as usize).map(|j| start.wrapping_add_signed(step * j as isize)).unwrap_or(Steps::FILL)).collect(),
            ref steps => rows.iter().map(|&j| lookup(j as usize).and_then(|j| Steps::reach(0, steps, j)).unwrap_or(Steps::FILL)).collect(),
        }
    }
    /// The targets of the cells at frame positions `rows`, cell by cell. Single items are their offsets, which are the positions themselves where the cells lie in frame order. Wider cells are a walk, a table of the cells' starts followed by the cells' own steps, which reads and writes cells whose items lie a stride apart, as columns' do, in tiles.
    fn targets(&self, rows: &[u32]) -> Targets {
        self.at_starts(self.starts(rows, Some))
    }
    /// The cells at `starts`, with a fill start reading a whole cell of fills. Transpose's cell steps start at 0, so singleton cells need only their starts.
    fn at_starts(&self, starts: Vec<usize>) -> Targets {
        let steps = &self.steps[self.rank..];
        if steps.iter().all(|s| s.len() == 1) { return Targets::Offsets(starts); }
        let starts = rustymath::gather::Steps::table(starts);
        Targets::Walk { base: 0, steps: std::iter::once(starts).chain(steps.iter().cloned()).collect() }
    }
    /// The selection of the cells at frame positions `rows`, as a list (`listed`).
    fn selection(&self, rows: &[u32], span: &Span) -> Result<Selection, Error> {
        Ok(Selection { frame: ResultFrame::Array(listed(&self.frame(), rows, &self.cells(), span)?), targets: self.targets(rows) })
    }
    /// The cells at frame positions `rows` of `array`, as a list (`listed`).
    fn read(&self, array: &Value, rows: &[u32], cx: &Context<'_>) -> Result<Value, Error> { self.selection(rows, cx.span)?.read(array, cx) }
}

/// The layout of the cells at positions `rows` of `frame`, as a list of cells laid out by `cell`. A frame of one axis keeps its name and the keys at `rows`.
fn listed(frame: &crate::array::Layout, rows: &[u32], cell: &crate::array::Layout, span: &Span) -> Result<crate::array::Layout, Error> {
    let list = if frame.shape().len() == 1 { frame.select(0, rows.iter().map(|&j| Some(j as usize))).error_at(span, "invalid selection")? } else { crate::array::Layout::from(vec![rows.len()]) };
    Ok(list.concat(cell))
}

/// The `len` cells of the frame grouped by the case of `cases` that each picks. `mapping` reads each cell's value from `selector`, and `None` reads them in place, where a Boolean is its own case number. A NaN, or a key that the selector lacks, picks no case, and those cells form a group after the cases.
fn picked(selector: &Value, mapping: Option<&Mapping>, len: usize, cases: usize, cx: &Context<'_>) -> Result<rustymath::search::Groups, Error> {
    use {crate::search::grouped_by, rustymath::gather::Steps};
    match (mapping, selector.as_items()) {
        (None, crate::array::Items::Compact(rustymath::buffer::View::Booleans(mask))) if cases > 1 => grouped_by(mask, cases, usize::from, cx),
        _ => {
            let values = crate::primitive::positions(selector, cases, cx)?;
            let keys = match mapping {
                None => values,
                Some(m) => (0..len).map(|j| m.get(j).map_or(Steps::FILL, |i| values[i])).collect(),
            };
            grouped_by(&keys, cases + usize::from(keys.contains(&Steps::FILL)), |p| p.min(cases), cx)
        }
    }
}

/// The map from each of the `len` positions of `frame` to a position of `input`, whose axes lie along the frame's in order, as agreement pairs them. An axis of length 1 stretches across the frame. Where both axes have keys, each position looks its key up (`Mapping::new`): a key that `input` lacks gives no position, and a key that only `input` has is never read. Any other axis needs the frame's length, or the call is a LENGTH ERROR with the message `mismatch`.
fn lookup(input: &crate::array::Layout, frame: &crate::array::Layout, len: usize, mismatch: &str, span: &Span) -> Result<Mapping, Error> {
    let rank = frame.shape().len();
    if (0..rank).any(|a| !(input.keys(a).is_some() && frame.keys(a).is_some()) && ![frame.shape()[a], 1].contains(&input.shape()[a])) {
        return Err(span.error(Kind::Length, mismatch));
    }
    Ok(Mapping::new(input, &(0..rank).collect::<Vec<_>>(), frame, len))
}

/// A left argument laid over a frame. Its axes pair with the frame's by name and then from the front, and each frame position reads the cell that `lookup` maps it to. A position whose key the argument lacks reads a cell of fills.
struct Laid { value: Value, framed: Framed, lookup: Mapping, frame: crate::array::Layout }

impl Laid {
    fn new(x: &Value, frame: &crate::array::Layout, span: &Span) -> Result<Self, Error> {
        const MISMATCH: &str = "the left argument doesn't line up with the right argument";
        let axes = rustymath::agreement::Agreement::paired_axes(x.layout(), frame, true);
        if axes.iter().any(|&a| a >= x.shape().len()) { return Err(span.error(Kind::Length, MISMATCH)); }
        let framed = Framed::new(x.layout(), &axes, span)?;
        let lookup = lookup(&framed.frame(), frame, frame.shape().iter().product(), MISMATCH, span)?;
        Ok(Self { value: x.clone(), framed, lookup, frame: frame.clone() })
    }

    /// The cells for frame positions `rows`, as a list that keeps the frame's keys at `rows` (`listed`).
    fn cells(&self, rows: &[u32], cx: &Context<'_>) -> Result<Value, Error> {
        let starts = self.framed.starts(rows, |j| self.lookup.get(j));
        let layout = listed(&self.frame, rows, &self.framed.cells(), cx.span)?;
        Selection { frame: ResultFrame::Array(layout), targets: self.framed.at_starts(starts) }.read(&self.value, cx)
    }

    /// The argument laid over the whole frame: its cells in frame order, with the frame's axes first.
    fn aligned(&self, cx: &Context<'_>) -> Result<Value, Error> {
        let len = self.frame.shape().iter().product::<usize>();
        self.cells(&(0..len as u32).collect::<Vec<_>>(), cx)
    }
}

/// The layout of `Cells::chosen`'s result: the frame of `framed`, whose axes are `axes` of the argument, with cells laid out by `cell`. Where the cells have the rank of the argument's cells, the frame's axes keep their places in the argument. Otherwise they come first, as a frame does in Rank. Also gives the positions of the frame's axes in the result.
fn placed(framed: &Framed, axes: &[usize], cell: &crate::array::Layout, span: &Span) -> Result<(crate::array::Layout, Vec<usize>), Error> {
    let first = framed.frame().concat(cell);
    let n = framed.layout.shape().len();
    if cell.shape().len() != n - axes.len() { return Ok((first, (0..axes.len()).collect())); }
    let plan = rustymath::structure::transpose(&first, &fronted(axes, n)).at(span)?;
    Ok((plan.layout.into(), axes.to_vec()))
}

/// `operand` called on prototypes, as Each calls it for a frame with no cells. A call that fails gives 0, so the result is empty with prototype 0, as in J.
fn on_prototypes(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    match operand.call_prototype(left, right, cx) {
        Err(e) if super::session::Catch::All.catches(&e.kind) => Ok(Bound::from(Value::Number(Numeric::current().float(0.0)))),
        result => result,
    }
}

/// The order of the axes of an array of rank `n` with the axes `first` moved to the front, in their order, and the others after them, in theirs.
fn fronted(first: &[usize], n: usize) -> Vec<usize> { first.iter().copied().chain((0..n).filter(|a| !first.contains(a))).collect() }

/// The destination of each axis, as `⍉` takes it, that puts an array's axes in `order`.
fn destination(order: &[usize]) -> Vec<usize> {
    let mut destination = vec![0; order.len()];
    for (j, &a) in order.iter().enumerate() { destination[a] = j; }
    destination
}

/// `f@g` for a function `g`: `f⍥g`, with the result written back. A `g` that selects writes it back where it took it from. Any other `g` is undone by its inverse.
fn under_function(f: &Function, g: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let result = over(f, g, left, right, cx)?.array(cx.span)?;
    let (labels, labelled) = crate::selection::Labels::new(right, cx.span)?;
    match g.select(None, &labelled, Some(right), SelectionKind::Item, cx)? {
        Some((selected, kind)) => {
            let (selection, values) = labels.replacements(&selected, &result, kind, cx.span)?;
            Ok(Bound::from(selection.write(right, &values, cx)?))
        }
        None if g.discards(false) => Err(cx.span.domain_error("Under can't use the inverse of a function that drops items")),
        None => g.inverse(cx.span)?.call(None, &result, cx),
    }
}

/// `f⍥g`: `f` applied to `g` of each argument.
fn over(f: &Function, g: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let y = g.call_array(None, right, cx)?;
    let x = left.map(|x| g.call_array(None, x, cx)).transpose()?;
    f.call(x.as_ref(), &y, cx)
}

/// `f⌺spec`: `f` on each window of `right`, with the window's border as its left argument, mixed as Rank mixes its results. With no windows, `f` runs on prototypes, as Each's operand does.
fn stencil(f: &Function, spec: &Value, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let (sizes, movements) = crate::primitive::window_spec(spec, cx.span)?;
    if sizes.is_empty() { return Err(cx.span.domain_error("stencil needs at least one axis")); }
    if sizes.iter().chain(&movements).any(|&n| n <= 0) { return Err(cx.span.domain_error("stencil sizes and movements must be positive")); }
    let windows = Windows::new(&sizes, &movements, |_| true, right.shape()).at(cx.span)?;
    generated_len(windows.cell()).at(cx.span)?;
    let count = generated_len(windows.frame()).at(cx.span)?;
    let layout = crate::array::Layout::from(windows.frame_layout(right.layout()).at(cx.span)?);
    let span = cx.span;
    // `starts` gives an empty frame one position, whose window is the prototype pair's right argument.
    let pair = |position| {
        let starts = windows.starts(position);
        let border = Value::floats(vec![starts.len()], Numeric::current().width, windows.border(&starts).into_iter().map(|n| n as f64)).unwrap();
        let window = remapped(right, windows.window(right.layout(), &starts).at(span)?).error_at(span, "invalid stencil window")?;
        Ok((Some(border), window))
    };
    mixed(each_pair(f, count, layout, pair, cx)?, span, "invalid stencil result")
}

fn power(operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    // A one-item list holding a predicate keeps every state of the until form.
    let listed = match operand {
        Operand::Value(a) if a.shape() == [1] && matches!(a.at(0), Value::Function(_)) => Some(Operand::from_value(Binding::from_element(a.at(0)))), _ => None
    };
    let history = listed.is_some();
    let operand = listed.as_ref().unwrap_or(operand);
    // `∞` runs until the state stops changing: until a state matches the one before within the convergence tolerance of each float width, whatever the pref. An iteration that settles into a cycle between adjacent floats then stops.
    let converge = |step: &Function, value: &mut Value, cx: &mut Context<'_>| -> Result<(), Error> {
        loop {
            let next = step.call_array(left, value, cx)?;
            let same = next.matches_within(value, Numeric::current().convergence(), cx)?;
            *value = next;
            if same { return Ok(()); }
        }
    };
    let mut value = right.clone();
    match operand {
        Operand::Value(count) if count.is_atom() => {
            let (negative, n) = checked_count(count, cx.span)?;
            let inverse;
            let f = if negative { inverse = f.inverse(cx.span)?; &inverse } else { f };
            if n == u64::MAX { converge(f, &mut value, cx)?; } else {
                for i in 0..n {
                    let result = f.call(left, &value, cx)?;
                    if i + 1 == n { return Ok(result); }
                    value = result.array(cx.span)?;
                }
            }
        }
        // An array of counts gives one item for each count, holding the state after that many steps.
        Operand::Value(count) => {
            let mut counts = Vec::new();
            power_counts(count, cx.span, &mut counts)?;
            counts.sort_unstable();
            counts.dedup();
            let mut states = HashMap::new();
            let (mut negative, mut steps, mut inverse) = (false, 0, None);
            for c in counts {
                if c.0 && !negative {
                    (value, negative, steps) = (right.clone(), true, 0);
                    inverse = Some(f.inverse(cx.span)?);
                }
                let step = inverse.as_ref().unwrap_or(f);
                if c.1 == u64::MAX { converge(step, &mut value, cx)?; }
                else { for _ in steps..c.1 { value = step.call_array(left, &value, cx)?; } }
                steps = c.1;
                states.insert(c, value.clone());
            }
            value = power_states(count, &states, right, cx.span)?;
        }
        Operand::Function(test) => {
            let mut states = if history { vec![value.clone()] } else { Vec::new() };
            loop {
                let next = f.call_array(left, &value, cx)?;
                let done = test.call_array(Some(&next), &value, cx)?;
                let done = done.boolean().error_at(cx.span, "power predicate must return a Boolean singleton")?;
                value = next;
                if history {
                    generated_len(&[states.len() + 1]).at(cx.span)?;
                    states.push(value.clone());
                }
                if done { break; }
            }
            if history {
                let items = states.iter().map(|s| s.enclose()).collect::<Result<Vec<_>, _>>().error_at(cx.span, "history result is too large")?;
                value = Value::assemble(&[items.len()], &items, &right.enclose().error_at(cx.span, "history result is too large")?)
                    .error_at(cx.span, "history result is too large")?;
            }
        }
    }
    Ok(Bound::from(value))
}

/// A count's direction and its number of steps, with `u64::MAX` for `∞`, or `None` for a count that is neither whole nor infinite.
pub(super) fn power_count(n: &crate::Number) -> Option<(bool, u64)> {
    if n.is_infinite() { return Some((n.as_float().is_some_and(|x| x < 0.), u64::MAX)); }
    n.integer().ok().map(|n| (n < 0, n.unsigned_abs()))
}

/// The count that the atom `e` holds.
fn checked_count(e: &Value, span: &Span) -> Result<(bool, u64), Error> {
    let Value::Number(n) = e else { return Err(span.domain_error("power counts must be numeric")) };
    power_count(n).ok_or_else(|| span.domain_error("power counts must be integral or infinite"))
}

/// The counts in a count array, through any nesting.
fn power_counts(count: &Value, span: &Span, counts: &mut Vec<(bool, u64)>) -> Result<(), Error> {
    for e in count.elements() { match &e { Value::Array(_) => power_counts(&e, span, counts)?, _ => counts.push(checked_count(&e, span)?) } }
    Ok(())
}

/// `count` with each count replaced by the state after that many steps, as one item. A nested count array gives a nested item.
fn power_states(count: &Value, states: &HashMap<(bool, u64), Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let items = count.elements().map(|e| {
        let state = match &e { Value::Array(_) => power_states(&e, states, right, span)?, _ => states[&checked_count(&e, span)?].clone() };
        state.enclose().error_at(span, "power result is too large")
    }).collect::<Result<Vec<_>, _>>()?;
    count.layout().assemble(&items, &right.enclose().error_at(span, "power result is too large")?).error_at(span, "power result is too large")
}

pub(super) fn inverse(f: &Function, bound: Option<(&Value, bool)>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), cx.span)? };
        g.inverse(cx.span)
    };
    let array = match f.node() {
        Primitive(p) => crate::primitive::inverse(*p, bound, right, None, cx),
        Ruled(_, Rules { inverse: Some(g), .. }) if first => return g.call(left, right, cx),
        Ruled(g, Rules { inverse: None, .. }) => return inverse(g, bound, right, cx),
        Composed(Under, [Operand::Function(g), h]) => return composition(Under, &[Operand::Function(operand_inverse(g)?), h.clone()], left, right, cx),
        Inverse(g) if first => return g.call(left, right, cx),
        Modified(Each, Operand::Function(g)) => return each(&operand_inverse(g)?, left, right, cx),
        Modified(Outer, Operand::Function(g)) => {
            let bound = left.ok_or_else(|| cx.span.domain_error("outer-product inverse needs a bound argument"))?;
            inverse_outer(g, bound, first, right, cx)
        }
        Modified(Commute, Operand::Value(k)) => {
            if !k.matches(right, cx)? { return Err(cx.span.domain_error("a constant function gives only its own value")); }
            Ok(right.clone())
        }
        Modified(Commute, Operand::Function(g)) => {
            if let Some((a, first)) = bound { return inverse(g, Some((a, !first)), right, cx); }
            use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
            // With one argument, `(g↢h)⍨` is the hook `g↢h`.
            if matches!(g.node(), Composed(After, [Operand::Function(_), Operand::Function(_)])) { return inverse(g, None, right, cx); }
            let Primitive(p) = g.node() else { return Err(cx.span.domain_error("this commute has no known inverse")); };
            match p {
                P::Arithmetic(Arithmetic::Plus) => {
                    P::Arithmetic(Arithmetic::Divide).call(Some(right), &Value::integer(2), cx)
                }
                P::Arithmetic(Arithmetic::Times) => P::Math(Math::Power).call(Some(right), &crate::primitive::float_like(0.5, right), cx),
                P::Math(Math::Floor | Math::Ceiling) | P::Identity(_) => Ok(right.clone()),
                P::Math(Math::Lcm | Math::Gcd) => {
                    if !p.call(Some(right), right, cx)?.matches(right, cx)? { return Err(cx.span.domain_error("no argument gives this result")); }
                    Ok(right.clone())
                }
                _ => Err(cx.span.domain_error("this commute has no known inverse")),
            }
        }
        Composed(Before, [Operand::Value(a), Operand::Function(g)]) if bound.is_none() => { return inverse(g, Some((a, true)), right, cx); }
        Composed(After, [Operand::Function(g), Operand::Value(a)]) if bound.is_none() => { return inverse(g, Some((a, false)), right, cx); }
        Composed(After, [Operand::Function(g), Operand::Function(h)]) => {
            let Some((a, first)) = bound else {
                // One argument makes a hook, `⍵ g h ⍵`. Only `×↢*` has a known inverse.
                use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
                if matches!(g.node(), Primitive(P::Arithmetic(Arithmetic::Times))) && matches!(h.node(), Primitive(P::Math(Math::Power))) {
                    return crate::primitive::lambert_w(right, cx).map(Bound::from);
                }
                return Err(cx.span.domain_error("this hook has no known inverse"));
            };
            if !first { let fixed = h.call_array(None, a, cx)?; return inverse(g, Some((&fixed, false)), right, cx); }
            let y = inverse(g, bound, right, cx)?.array(cx.span)?;
            return h.inverse(cx.span)?.call(None, &y, cx);
        }
        Composed(Atop, [Operand::Function(g), Operand::Function(h)]) => {
            let y = g.inverse(cx.span)?.call_array(None, right, cx)?;
            return inverse(h, bound, &y, cx);
        }
        Composed(Over, [Operand::Function(g), Operand::Function(h)]) => {
            let fixed = left.map(|a| h.call_array(None, a, cx)).transpose()?;
            let y = inverse(g, fixed.as_ref().map(|a| (a, first)), right, cx)?.array(cx.span)?;
            return h.inverse(cx.span)?.call(None, &y, cx);
        }
        Composed(Before, [Operand::Function(g), Operand::Function(h)]) if bound.is_some() => {
            if first { let fixed = g.call_array(None, left.unwrap(), cx)?; return inverse(h, Some((&fixed, true)), right, cx); }
            let y = inverse(h, bound, right, cx)?.array(cx.span)?;
            return g.inverse(cx.span)?.call(None, &y, cx);
        }
        Composed(Valences, [Operand::Function(g), Operand::Function(h)]) => {
            return if bound.is_some() { inverse(h, bound, right, cx) } else { inverse(g, None, right, cx) };
        }
        Composed(Power, [Operand::Function(g), Operand::Value(count)]) if first && count.is_atom() => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, cx)?;
            return power(&[Operand::Function(g.clone()), Operand::Value(count)], left, right, cx);
        }
        Composed(Rank, [Operand::Function(g), Operand::Value(ranks)]) => {
            let mut ranks = ranks.clone();
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items).error_at(cx.span, "invalid inverse ranks")?;
            }
            return rank(&operand_inverse(g)?, &ranks, left, right, cx);
        }
        FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, None, left, right, cx),
        FunctionNode::Axis(g, axis) => {
            let mut g = g;
            while let FunctionNode::Axis(inner, _) = g.node() { g = inner; }
            match g.node() {
                Primitive(p) => crate::primitive::inverse(*p, bound, right, Some(axis), cx),
                FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, Some(axis), left, right, cx),
                _ => Err(cx.span.domain_error("this axis-qualified function has no known inverse")),
            }
        }
        System(f) if first => crate::system::inverse(f, left, right, cx),
        _ => Err(cx.span.domain_error("this function has no known inverse")),
    }?;
    Ok(Bound::from(array))
}

fn inverse_outer(f: &Function, bound: &Value, first: bool, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank { return Err(cx.span.domain_error("outer-product inverse needs a nonempty matching bound frame")); }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(cx.span.domain_error("outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape).at(cx.span)?;
    let layout = right.layout().axes(if first { split..right.shape().len() } else { 0..split });
    let mut keys = vec![None; right.shape().len()];
    for axis in 0..rank { keys[if first { axis } else { split + axis }] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(right, &keys, false).error_at(cx.span, "outer-product bound keys must agree")?;
    let mut result: Vec<Value> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Value(bound.at(j));
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse =
            Function::new(FunctionNode::Composed(if first { OperatorKind::Before } else { OperatorKind::After }, operands), cx.span)?.inverse(cx.span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value, cx)?;
            if j == 0 { result.push(candidate); }
            else if !result[i].matches(&candidate, cx)? { return Err(cx.span.domain_error("outer-product cells do not have a consistent inverse")); }
        }
    }
    let prototype = result[0].prototype();
    if count == 0 { result.clear(); }
    layout.collect(result, prototype).error_at(cx.span, "invalid outer-product inverse")
}

fn inverse_scan(f: &Function, h: FoldKind, axis: Option<&Value>, seed: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let axis = scan_axis(h, axis, right, cx.span)?;
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    let inverse = f.inverse(cx.span)?;
    if right.is_atom() { return inverse.call_array(seed, right, cx); }
    if let Some(p) = f.as_primitive() {
        if let Some(result) = crate::pervasive::inverse_scan(p, right, seed, &axis) {
            return result.with_layout(right.layout().clone()).error_at(cx.span, "invalid inverse scan");
        }
    }
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous = if j == 0 { seed.unwrap().clone() } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = inverse.call_array(Some(&previous), &right.at(offset), cx)?;
            }
        }
    }
    right.layout().collect(data, || right.prototype()).error_at(cx.span, "invalid inverse scan")
}

/// The Key `derived`, `f⌸`. The idiom it may hold (`idioms`) counts each group, or folds each group of a vector, without calling `f`. A fold that `pervasive::fold_groups` can't compute takes the general path, which reuses the same classes.
pub(super) fn key(derived: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    use idioms::Idiom;
    let FunctionNode::Modified(_, Operand::Function(f)) = derived.node() else { unreachable!("a Key holds its operand") };
    let keys = left.unwrap_or(right);
    if keys.is_unit() || right.is_unit() { return Err(cx.span.error(Kind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(cx.span.error(Kind::Length, "key arguments must have equal tallies")); }
    let key_cells = crate::search::Cells::of(keys, keys.shape().len() - 1).error_at(cx.span, "invalid key cells")?;
    let idiom = derived.node.idiom.as_ref();
    if let Some(counts @ Idiom::Counts) = idiom.filter(|_| keys.shape()[0] > 0) {
        let sizes = crate::search::class_counts(&key_cells, cx)?;
        counts.taken(derived, cx);
        return Ok(Bound::from(Value::positions(vec![sizes.len()], keys.shape()[0] + 1, sizes.into_iter()).error_at(cx.span, "invalid key result")?));
    }
    let mut classes = crate::search::grouped(&key_cells, cx)?;
    let count = classes.firsts.len();
    if let Some(folds @ Idiom::Folds(p)) = idiom.filter(|_| left.is_some() && count > 0 && right.shape().len() == 1 && !crate::derivative::active_in(cx, right)) {
        if let Some(result) = crate::pervasive::fold_groups(*p, right, &classes) {
            folds.taken(derived, cx);
            return Ok(Bound::from(result));
        }
    }
    // A monadic Key's values are the positions along `right`, which are each group's rows themselves unless that axis has keys.
    let values = match left {
        Some(_) => Some(right.clone()),
        None if right.keys(0).is_some() => Some(crate::keyed::selectors(right, &[0]).error_at(cx.span, "invalid group indices")?), None => None
    };
    let width = values.as_ref().map_or(Ok(1), |v| generated_len(&v.shape()[1..])).at(cx.span)?;
    let select = |rows: &[u32]| {
        let Some(values) = &values else { return Value::positions(vec![rows.len()], right.shape()[0], rows.iter().copied()) };
        let layout = values.layout().select(0, rows.iter().map(|&r| Some(r as usize)))?;
        let mut data = crate::array::Gather::new(&[values], rows.len() * width);
        data.rows(values, rows, width);
        data.finish(layout, || values.prototype())
    };
    if count == 0 {
        let cells = keys.cells(keys.shape().len() - 1).error_at(cx.span, "invalid key cells")?;
        let x = cells.prototype().error_at(cx.span, "invalid key prototype")?;
        let y = select(&[]).error_at(cx.span, "invalid key group")?;
        let result = on_prototypes(f, Some(&if cells.shape().is_empty() { x.at(0) } else { x }), &y, cx)?.array(cx.span)?;
        return Ok(Bound::from(Value::assemble(&[0], &[], &result).error_at(cx.span, "invalid key result")?));
    }
    let firsts = std::mem::take(&mut classes.firsts);
    let mut results = Vec::with_capacity(count);
    let mut apply = |g: usize, y: Value, cx: &mut Context<'_>| -> Result<(), Error> {
        results.push(f.call_array(Some(&key_cells.get(firsts[g])), &y, cx)?);
        Ok(())
    };
    match &values {
        None => {
            for (g, y) in crate::search::class_positions(classes.groups, right.shape()[0]).error_at(cx.span, "invalid key group")?.into_iter().enumerate() {
                apply(g, y, cx)?
            }
        }
        Some(_) => { for (g, rows) in classes.groups.each().enumerate() { apply(g, select(rows).error_at(cx.span, "invalid key group")?, cx)? } }
    }
    Ok(Bound::from(Value::assemble(&[count], &results, &results[0]).error_at(cx.span, "invalid key result")?))
}

/// The primitive in `f`, when `f` is a primitive whose form for this valence is a pervasive function. Each and Outer call such a primitive on whole arrays.
fn pervasive_primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    f.as_primitive().filter(|p| p.pervasive(dyadic))
}

pub(super) fn outer(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| cx.span.error(ErrorKind::Syntax, "outer product needs a left argument"))?;
    if left.is_atom() && right.is_atom() { return operand.call(Some(left), right, cx); }
    if let Some(p) = pervasive_primitive(operand, true) { return Ok(Bound::from(p.outer(left, right, cx)?)); }
    let layout = left.layout().concat(right.layout());
    let len = generated_len(layout.shape()).at(cx.span)?;
    // With no pairs, an empty argument gives its prototype and a nonempty one its first item.
    let item = |a: &Value, i| if a.is_empty() { a.prototype() } else { a.at(i) };
    let columns = right.len().max(1);
    each_pair(operand, len, layout, |i| Ok((Some(item(left, i / columns)), item(right, i % columns))), cx)
}

fn inner(f: &Function, g: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    use crate::number::Arithmetic::{Plus, Times};
    let Some(left) = left else { return expansion(f, g, right, cx) };
    // The tangent of `A+.×B` is `(tA+.×B)+A+.×tB`, through the same product kernels.
    if cx.session.transforming() && matches!(f.as_primitive().zip(g.as_primitive()), Some((Primitive::Arithmetic(Plus), Primitive::Arithmetic(Times)))) {
        // A record reaches here only as a dual's tangent, which the terms below record with the primal's layout (`linear::product`).
        if left.linear().is_some() || right.linear().is_some() { return Err(cx.span.error(crate::ErrorKind::Unsupported, "∂ can't transpose this product")); }
        if let Some(tag) = crate::derivative::tag(left).max(crate::derivative::tag(right)) {
            let (a, ta) = crate::derivative::split(left, tag);
            let (b, tb) = crate::derivative::split(right, tag);
            let r = inner(f, g, Some(&a), &b, cx)?.array(cx.span)?;
            let mut t = ta.map(|ta| crate::derivative::product(&ta, &b, cx)).transpose()?;
            if let Some(tb) = tb {
                let term = crate::derivative::product(&a, &tb, cx)?;
                t = Some(match t { Some(t) => Primitive::Arithmetic(crate::number::Arithmetic::Plus).call(Some(&t), &term, cx)?, None => term });
            }
            return Ok(Bound::from(crate::derivative::dual(tag, r, t.expect("a dual argument has a tangent"))));
        }
    }
    let nx = left.shape().last().copied().unwrap_or(1);
    let ny = right.shape().first().copied().unwrap_or(1);
    let positions =
        Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), 0).at(cx.span)?;
    if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(cx.span.error(Kind::Length, "product contraction lengths must agree")); }
    let n = if left.is_singleton() { ny } else { nx };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = &right.shape()[usize::from(!right.is_unit())..];
    let layout = left.layout().contracted(right.layout());
    let size = generated_len(layout.shape()).at(cx.span)?;
    let rows = generated_len(xf).at(cx.span)?;
    let cols = generated_len(yf).at(cx.span)?;
    generated_len(&[n.max(1), cols.max(1)]).at(cx.span)?;
    let active = crate::derivative::either_active(cx, Some(left), right);
    if let (Some((pf, pg)), false) = (f.as_primitive().zip(g.as_primitive()), active) {
        if nx == ny && n > 0 && size > 0 && !left.is_unit() && !right.is_unit() && matches!(positions, Mapping::Linear(1)) {
            if let Some(result) = crate::pervasive::inner(pf, pg, left, right, [rows, n, cols], layout.shape().to_vec()) {
                if layout.shape().is_empty() { return Ok(Bound::from(result.at(0))); }
                return Ok(Bound::from(result.with_layout(layout).error_at(cx.span, "invalid inner product result")?));
            }
        }
    }
    // An empty result, or an empty contraction, calls the operands only for prototypes, so in prototype mode.
    let item = |a: &Value, offset| if n == 0 || a.is_empty() { a.prototype() } else { a.at(if a.is_singleton() { 0 } else { offset }) };
    let mut results = cx.prototype_mode(size == 0, |cx| {
        let mut results = Vec::with_capacity(size.max(1));
        for i in 0..rows.max(1) {
            let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
            for k in 0..n.max(1) {
                for (j, column) in columns.iter_mut().enumerate() {
                    let rk = if n == 0 { 0 } else { positions.index(k) };
                    let (x, y) = (item(left, i * nx + k), item(right, rk * cols + j));
                    column.push(cx.prototype_mode(n == 0, |cx| g.call_array(Some(&x), &y, cx))?);
                }
            }
            for column in columns {
                let paired = if n == 0 { Value::empty(vec![0], column[0].prototype()) } else { Value::new(vec![n], column) }
                    .error_at(cx.span, "invalid product cell")?;
                results.push(fold(f, FoldKind::REDUCE, None, None, &paired, cx)?);
            }
        }
        Ok::<_, Error>(results)
    })?;
    if layout.shape().is_empty() { return Ok(Bound::from(results.remove(0))); }
    let prototype = results[0].fill();
    if size == 0 { results.clear(); }
    let result = layout.collect(results, prototype);
    Ok(Bound::from(result.error_at(cx.span, "invalid inner product result")?))
}

/// Monadic `f.g`, as J defines it: the expansion of a matrix by minors along its first column. A vector is one column, and an atom one item. `-.×` is the determinant and `+.×` the permanent, which `idioms` computes by faster algorithms.
fn expansion(f: &Function, g: &Function, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [m, n] = matrix(right).ok_or_else(|| cx.span.error(Kind::Rank, "monadic f.g needs a matrix, a vector or a number"))?;
    if crate::derivative::active_in(cx, right) { return Err(cx.span.error(ErrorKind::Unsupported, "∂ differentiates monadic f.g only as -.×")); }
    if m > 64 { return Err(cx.span.error(Kind::Limit, "monadic f.g expands at most 64 rows")); }
    let items: Vec<Value> = (0..m * n).map(|i| right.at(i)).collect();
    let rows = if m == 0 { 0 } else { u64::MAX >> (64 - m) };
    expand(f, g, &items, [m, n], rows, &right.prototype(), &mut HashMap::new(), cx).map(Bound::from)
}

/// The rows and columns that monadic `f.g` reads `right` as: a vector is one column, and a number one item.
pub(super) fn matrix(right: &Value) -> Option<[usize; 2]> { match *right.shape() { [] => Some([1, 1]), [m] => Some([m, 1]), [m, n] => Some([m, n]), _ => None } }

/// The expansion of the rows in the set `rows` of the `m`×`n` matrix `items`, from column `m` minus the size of `rows` on. While columns are left, it is `f.g` between that column's items in `rows` and the expansions without each of those rows. With no columns left, it is `g/` of no items. `memo` holds the expansion of each set of rows already expanded.
fn expand(
    f: &Function, g: &Function, items: &[Value], [m, n]: [usize; 2], rows: u64, prototype: &Value, memo: &mut HashMap<u64, Value>, cx: &mut Context<'_>
) -> Result<Value, Error> {
    if let Some(value) = memo.get(&rows) { return Ok(value.clone()); }
    cx.check()?;
    let vector = |items: Vec<Value>, cx: &Context<'_>| {
        if items.is_empty() { Value::empty(vec![0], prototype.clone()) } else { Value::new(vec![items.len()], items) }.error_at(cx.span, "invalid expansion")
    };
    let column = m - rows.count_ones() as usize;
    let value = if column == n { fold(g, FoldKind::REDUCE, None, None, &vector(vec![], cx)?, cx)? } else {
        let set: Vec<usize> = (0..m).filter(|&i| rows >> i & 1 == 1).collect();
        let first = vector(set.iter().map(|&i| items[i * n + column].clone()).collect(), cx)?;
        let minors = set.iter().map(|&i| expand(f, g, items, [m, n], rows & !(1 << i), prototype, memo, cx)).collect::<Result<Vec<_>, _>>()?;
        let minors = vector(minors, cx)?;
        inner(f, g, Some(&first), &minors, cx)?.array(cx.span)?
    };
    memo.insert(rows, value.clone());
    Ok(value)
}

/// The cell ranks that rank operand `ranks` gives, as `[monad, left, right]`. ∞ and ¯∞ become `isize::MAX` and `isize::MIN`, which `within` clamps.
fn cell_ranks(ranks: &Value, cx: &Context<'_>) -> Result<[isize; 3], Error> {
    if ranks.shape().len() > 1 { return Err(cx.span.error(Kind::Rank, "rank operand must be a unit or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(cx.span.error(Kind::Length, "rank operand needs one to three items")); }
    let ranks = ranks.elements().map(|e| match e {
            Value::Number(n) if n.is_infinite() => Ok(if n.as_float().is_some_and(f64::is_sign_positive) { isize::MAX } else { isize::MIN }),
            Value::Number(n) => {
                n.integer().map(|n| isize::try_from(n).unwrap_or(if n < 0 { isize::MIN } else { isize::MAX })).error_at(cx.span, "cell ranks must be integers")
            }
            _ => Err(cx.span.domain_error("cell ranks must be numeric")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match ranks.as_slice() { [r] => [*r, *r, *r], [q, r] => [*r, *q, *r], [p, q, r] => [*p, *q, *r], _ => unreachable!() })
}

/// The rank of the cells of cell rank `k` in an array or cell of rank `n`. A negative `k` counts down from `n`. ∞ gives `n`, and ¯∞ gives 0.
fn within(n: usize, k: isize) -> usize { if k < 0 { n.saturating_sub(k.unsigned_abs()) } else { n.min(k as usize) } }

pub(super) fn rank(operand: &Function, ranks: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [p, q, r] = cell_ranks(ranks, cx)?;
    let yr = within(right.shape().len(), if left.is_some() { r } else { p });
    let xr = left.map_or(0, |a| within(a.shape().len(), q));
    if let Some(result) = try_bulk(operand, left, right, [xr, yr], cx) { return result; }
    let framed = |a: &Value, rank| a.cells(rank)?.framed();
    let ys = framed(right, yr).error_at(cx.span, "invalid rank cells")?;
    let xs = left.map(|a| framed(a, xr)).transpose().error_at(cx.span, "invalid rank cells")?;
    mixed(paired(operand, xs.as_ref(), &ys, cx)?, cx.span, "invalid rank result")
}

/// A map's results, one for each position of its frame, mixed into one array whose numbers keep their kinds, as Rank and stencils mix them.
fn mixed(results: Bound, span: &Span, what: &str) -> Result<Bound, Error> {
    let Binding::Value(results) = results.value else { return Ok(results) };
    Ok(Bound::from(results.mix(crate::array::Widening::Items).error_at(span, what)?))
}

/// `operand⍤k` in calls on whole arguments, where they give what one call for each cell gives. `xr` and `yr` are the cell ranks. `None` where no rule applies. Rank then calls `operand` once for each cell.
fn try_bulk(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], cx: &mut Context<'_>) -> Option<Result<Bound, Error>> {
    // With no frame, the one cell is the whole argument.
    if yr == right.shape().len() && left.is_none_or(|x| xr == x.shape().len()) { return Some(operand.call(left, right, cx)); }
    if let Some(result) = stacked(operand, left, right, [xr, yr], cx) { return Some(result); }
    if let Some(result) = pervasive_cells(operand, left, right, [xr, yr], cx) { return Some(result); }
    if left.is_some() { return None; }
    cell_rule(operand, right, yr, cx).map(|result| result.map(Bound::from))
}

/// `f` on each rank-`k` cell of `y`, as calls on the whole of `y` along the cells' axes. The frame's axes keep their keys and names. `None` where no rule covers `f`.
fn cell_rule(f: &Function, y: &Value, k: usize, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    use {FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::{Atop, Before}};
    let frame = y.shape().len() - k;
    // Cell axes `from..from + count`, numbered as axes of the whole of `y`.
    let axes = |from: usize, count: usize| Value::integers(vec![count], (frame + from..frame + from + count).map(|a| a as i64).collect()).expect("axes are integers");
    match f.node() {
        N::Primitive(p) => {
            if p.extends_at(None, k) { return Some(p.call(None, y, cx)); }
            match p {
                Primitive::Reverse(true) if k > 0 => Some(p.call_axes(None, y, &axes(0, 1), cx)),
                Primitive::Ravel => Some(p.call_axes(None, y, &axes(0, k), cx)),
                Primitive::Tally => Some(constant(y, frame, &[], &[if k == 0 { 1 } else { y.shape()[frame] as i64 }], cx)),
                // The shape of a cell with named axes is keyed by the names.
                Primitive::Shape if y.layout().names().iter().skip(frame).all(Option::is_none) => {
                    Some(constant(y, frame, &[k], &y.shape()[frame..].iter().map(|&n| n as i64).collect::<Vec<_>>(), cx))
                }
                _ => None,
            }
        }
        // A pervasive primitive's fold or scan of each cell of a simple array is simple. Rank's mix of those results gives what the whole-array call gives.
        N::Fold(g, kind) if k > 0 && folds_whole(g, None, y) => {
            let axis = Value::integer((frame + kind.default_axis(k)) as i64);
            Some(if kind.scan { scan(g, *kind, Some(&axis), None, y, cx) } else { fold(g, *kind, Some(&axis), None, y, cx) })
        }
        N::Composed(Before, [V(a), F(g)]) if a.keys(0).is_none() => {
            let Some(p) = g.as_primitive() else { return None };
            if p.extends_at(Some(a), k) { return Some(p.call(Some(a), y, cx)); }
            // A left argument of take, drop or index has one item for each leading cell axis it applies to.
            let leading = match a.shape() { [] => 1, &[n] => n, _ => return None };
            match p {
                // A named selector stays on the cell path, because the whole array's frame axes could pair with its name.
                Primitive::Index if leading <= k && a.elements().any(|e| e.axis_names().iter().any(Option::is_some)) => None,
                Primitive::Take | Primitive::Drop | Primitive::Index if leading <= k => Some(p.call_axes(Some(a), y, &axes(0, leading), cx)),
                Primitive::Reverse(true) if a.shape().is_empty() && k > 0 => Some(p.call_axes(Some(a), y, &axes(0, 1), cx)),
                // A reshape that keeps a cell's items in order is a view.
                Primitive::Shape => {
                    let shape: Vec<usize> = a.elements().map(|e| e.as_number().and_then(|n| n.as_integer()).and_then(|n| usize::try_from(n).ok())).collect::<Option<_>>()?;
                    if shape.iter().product::<usize>() != y.shape()[frame..].iter().product::<usize>() { return None; }
                    Some(y.relaid(y.layout().axes(0..frame).concat(&crate::array::Layout::from(shape))).error_at(cx.span, "invalid cell reshape"))
                }
                _ => None,
            }
        }
        N::Composed(Atop, [F(g), F(h)]) => {
            // A fold of each cell's ravel folds all the cell axes at once. That takes folds in turn where the primitive allows them. Folds in turn reach sliding folds over windows.
            if let (N::Fold(op, kind), Some(Primitive::Ravel)) = (g.node(), h.as_primitive()) {
                if !kind.scan && k > 0 && folds_whole(op, None, y) { return Some(fold(op, *kind, Some(&axes(0, k)), None, y, cx)); }
            }
            let inner = match cell_rule(h, y, k, cx)? { Ok(inner) => inner, Err(e) => return Some(Err(e)) };
            let ranks = Value::integer((inner.shape().len() - frame) as i64);
            Some(rank(g, &ranks, None, &inner, cx).and_then(|result| result.array(cx.span)))
        }
        _ => None,
    }
}

/// An array with `y`'s first `frame` axes and their keys, then `tail`, holding `items` at each frame position. Monads whose result depends only on a cell's shape give this for every cell.
fn constant(y: &Value, frame: usize, tail: &[usize], items: &[i64], span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    let layout = y.layout().axes(0..frame).concat(&crate::array::Layout::from(tail.to_vec()));
    let count = crate::array::element_count(layout.shape()).at(span)?;
    Value::integers(layout.shape().to_vec(), items.iter().copied().cycle().take(count).collect()).and_then(|v| v.with_layout(layout)).error_at(span, "invalid cell shapes")
}

/// `f⍤k` in one call, where `f` is a pervasive primitive, or Rank nested down to one, as in `(-⍤1)⍤2 1`. A monad maps each item, whatever its cells. Each level of Rank pairs its arguments' frames as leading axes. So for a dyad, each argument gets length-1 axes after each level's frame, up to the other argument's frame at that level, and one call then pairs frames with frames and cells with cells. Some calls take the path through cells instead. Separate calls never pair axes by name, so arguments with axis names do. An empty argument does, because its result's shape comes from a call on prototype cells. So do arguments whose padded layouts don't agree. That path reports the error.
fn pervasive_cells(operand: &Function, left: Option<&Value>, right: &Value, ranks: [usize; 2], cx: &mut Context<'_>) -> Option<Result<Bound, Error>> {
    if left.into_iter().chain([right]).any(|a| a.is_empty() || !a.layout().names().is_empty()) { return None; }
    let (f, levels) = rank_levels(operand, left.is_some(), ranks, cx)?;
    let Some(x) = left else { return Some(f.call(None, right, cx)) };
    let (fx, fy) = (frames(x, &levels, 0), frames(right, &levels, 1));
    let (x, y) = (padded(x, &fx, &fy).ok()?, padded(right, &fy, &fx).ok()?);
    Agreement::new(x.layout(), y.layout()).ok()?;
    Some(f.call(Some(&x), &y, cx))
}

/// The pervasive primitive that `operand` is, or that Rank nested in `operand` comes down to, with each level's cell ranks as `[left, right]`, starting from `ranks`.
fn rank_levels<'a>(operand: &'a Function, dyadic: bool, ranks: [usize; 2], cx: &Context<'_>) -> Option<(&'a Function, Vec<[usize; 2]>)> {
    let (mut f, mut levels) = (operand, vec![ranks]);
    while pervasive_primitive(f, dyadic).is_none() {
        let FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(g), Operand::Value(k)]) = f.node() else { return None };
        let [m, q, r] = cell_ranks(k, cx).ok()?;
        let [xr, yr] = *levels.last()?;
        levels.push([within(xr, q), within(yr, if dyadic { r } else { m })]);
        f = g;
    }
    Some((f, levels))
}

/// The length of each level's frame in `a`: the axes between the cells of the level above and the level's own cells. `side` is 0 for the left argument's cell ranks, 1 for the right's.
fn frames(a: &Value, levels: &[[usize; 2]], side: usize) -> Vec<usize> {
    let mut above = a.shape().len();
    levels.iter().map(|l| std::mem::replace(&mut above, l[side]) - l[side]).collect()
}

/// A view of `a` with length-1 axes after each level's frame, up to the length of the other argument's frame at that level. `frame` and `other` hold those lengths. Length-1 axes after `a`'s last own axis change nothing, because leading-axis agreement extends it over trailing axes.
fn padded(a: &Value, frame: &[usize], other: &[usize]) -> Result<Value, ErrorKind> {
    let (mut axes, mut next) = (vec![], 0);
    for (&n, &m) in frame.iter().zip(other) {
        axes.extend((next..next + n).map(Some));
        axes.extend(std::iter::repeat_n(None, m.saturating_sub(n)));
        next += n;
    }
    axes.extend((next..a.shape().len()).map(Some));
    while axes.last() == Some(&None) { axes.pop(); }
    if axes.len() == a.shape().len() { return Ok(a.clone()); }
    let shape: Vec<_> = axes.iter().map(|&s| s.map_or(1, |i| a.shape()[i])).collect();
    a.relaid(crate::array::Layout::from(shape).with_keys(axes.iter().map(|&s| s.and_then(|i| a.keys(i).cloned())).collect())?)
}

/// `operand⍤2` on a whole stack of matrices in one call, where `operand` is dyadic `+.×` or `⌹`, monadic `⌹` or `-.×`, or `•decompose` with its kind bound to it or as one whole left cell. `xr` and `yr` are the cell ranks. `None` where it takes the general path of one call for each cell.
fn stacked(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], cx: &mut Context<'_>) -> Option<Result<Bound, Error>> {
    use crate::{number::Arithmetic, primitive::{stacked as matrices, stacked_decompositions, Stacked}};
    // The stacked kernels read compact storage, which a dual doesn't have: its cells go through the operand one by one (`derivative`).
    if yr != 2 || crate::derivative::either_active(cx, left, right) { return None; }
    let decompose = |f: &Function| matches!(f.node(), FunctionNode::System(s) if s.name == "•decompose");
    let kind = match (operand.node(), left) {
        (FunctionNode::System(_), Some(kind)) if decompose(operand) && xr == kind.shape().len() => Some(kind),
        (FunctionNode::Composed(OperatorKind::Before, [Operand::Value(kind), Operand::Function(f)]), None) if decompose(f) => Some(kind), _ => None
    };
    if let Some(kind) = kind { return Some(stacked_decompositions(kind, right, cx)?.map(Bound::from)); }
    if left.is_some() && xr != 2 { return None; }
    let op = match (operand.node(), left) {
        (FunctionNode::Primitive(Primitive::MatrixDivide), None) => Stacked::Inverse,
        (FunctionNode::Primitive(Primitive::MatrixDivide), Some(_)) => Stacked::Solve,
        (FunctionNode::Composed(OperatorKind::Product, [Operand::Function(f), Operand::Function(g)]), _) => match (f.as_primitive().zip(g.as_primitive()), left) {
            (Some((Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times))), Some(_)) => Stacked::Product,
            (Some((Primitive::Arithmetic(Arithmetic::Minus), Primitive::Arithmetic(Arithmetic::Times))), None) => Stacked::Determinant,
            _ => return None,
        },
        _ => return None,
    };
    matrices(op, left, right, cx).map(|r| r.map(Bound::from))
}

/// `f⍠spec`: a primitive with its own axis form, a reduce or scan along an axis, or `f` on the cells that the axes make.
pub(super) fn along_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    match f.node() {
        FunctionNode::Primitive(p) if p.takes_axes(left.is_some()) => Ok(Bound::from(p.call_axes(left, right, spec, cx)?)),
        FunctionNode::Fold(operand, kind) => Ok(Bound::from(fold(operand, *kind, Some(spec), left, right, cx)?)), _ => cell_axes(f, spec, left, right, cx)
    }
}

/// Apply `f` to the cells made of the selected axes. The other axes form the frame. The selected axes must exist in the higher-rank argument. An argument that lacks one of them is one whole cell.
pub(super) fn cell_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let target = match left { Some(x) if x.shape().len() > right.shape().len() => x, _ => right };
    let order = target.shape().len();
    let spec = crate::primitive::resolve_axes(spec, target, cx.span)?;
    let selected = crate::primitive::axes(&spec, order, cx)?;
    let others = |n: usize| (0..n).filter(|i| !selected.contains(i));
    // Move the selected axes to the end, so that rank sees them as the cells.
    let cells = |x: &Value| -> Result<(Value, usize), Error> {
        let n = x.shape().len();
        if selected.iter().any(|&i| i >= n) { return Ok((x.clone(), n)); }
        Ok((moved_last(x, &selected, cx)?, selected.len()))
    };
    let (y, yr) = cells(right)?;
    let x = left.map(cells).transpose()?;
    let ranks: Vec<_> = x.iter().map(|(_, r)| *r as i64).chain([yr as i64]).collect();
    let ranks = Value::integers(vec![ranks.len()], ranks).error_at(cx.span, "invalid cell ranks")?;
    let result = rank(f, &ranks, x.as_ref().map(|(x, _)| x), &y, cx)?.array(cx.span)?;
    // Result cells of the selected rank return to the selected axes. Other result cells start at the first selected axis.
    let frame = order - selected.len();
    let cell = result.shape().len().saturating_sub(frame);
    let destination: Vec<_> = if cell == selected.len() { others(order).chain(selected.iter().copied()).collect() } else {
        let start = selected.first().map_or(frame, |&s| others(order).take_while(|&i| i < s).count());
        (0..frame + cell).map(|j| {
            if j < start { j } else if j < frame { j + cell } else { start + j - frame }
        }).collect()
    };
    Ok(Bound::from(move_axes(&result, &destination, cx)?))
}

/// Transpose so that axis `i` of `x` becomes axis `destination[i]`.
fn move_axes(x: &Value, destination: &[usize], span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    if rustymath::gather::counts_up(destination) { return Ok(x.clone()); }
    crate::primitive::transpose_axes(destination.to_vec(), x, span)
}

/// `x` with the `selected` axes moved to the end, in their order.
fn moved_last(x: &Value, selected: &[usize], span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    let n = x.shape().len();
    move_axes(x, &destination(&(0..n).filter(|i| !selected.contains(i)).chain(selected.iter().copied()).collect::<Vec<_>>()), span)
}

/// Calls `operand` on the argument pairs `pair(0)`, `pair(1)` and so on up to `len`, and assembles the results in `layout`. With no pairs it calls the operand once on prototypes (`on_prototypes`), on `pair(0)`, for the result's prototype. A call that gives no result contributes `⍬`. The calls run as a map, whose cells each draw random numbers from a stream of their own (`Context::mapped`).
fn each_pair(
    operand: &Function, len: usize, layout: crate::array::Layout, pair: impl Fn(usize) -> Result<(Option<Value>, Value), Error>, cx: &mut Context<'_>
) -> Result<Bound, Error> {
    cx.mapped(|cx| {
        let empty = len == 0;
        let mut data = Gather::items(len);
        let mut first = None;
        for i in 0..len.max(1) {
            cx.cell(i);
            let (x, y) = pair(i)?;
            let result = if empty { on_prototypes(operand, x.as_ref(), &y, cx)? } else { operand.call(x.as_ref(), &y, cx)? };
            let item = match result.value {
                Binding::Value(a) => a, Binding::Function(f) => Value::Function(f),
                Binding::NoResult => crate::syntax::zilde(Numeric::current().float(0.0)),
                _ => return Err(cx.span.error(ErrorKind::Syntax, "the operand must return an array, function or no result"))
            };
            if empty { first = Some(item) } else { data.add(item) }
        }
        let value = Binding::Value(data.finish(layout, || first.unwrap().fill()).error_at(cx.span, "invalid result")?);
        Ok(Bound::new(value))
    })
}

pub(super) fn each(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    if right.is_atom() && left.is_none_or(Value::is_atom) { return operand.call(left, right, cx); }
    if pervasive_primitive(operand, left.is_some()).is_some() { return operand.call(left, right, cx); }
    paired(operand, left, right, cx)
}

/// Calls `operand` on each pair of items that the frames' agreement pairs, through `each_pair`. An empty frame gives an empty result when the call on its prototype items fails, as in J.
fn paired(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(cx.span, "frames do not agree")?;
    each_pair(operand, agreement.len, agreement.layout.clone(), |i| Ok(agreement.values(left, right, i)), cx)
}

fn identity(operand: &Function, prototype: &Value, span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    use crate::primitive::Identity::*;
    let id = operand.as_primitive().and_then(|p| p.info().dyad.and_then(|d| d.identity));
    let id = id.ok_or_else(|| span.domain_error("this function has no reduction identity"))?;
    let number = |n: f64| Ok(crate::primitive::float_like(n, prototype));
    match (id, prototype) {
        (Empty(first), _) => {
            let axis = (!first).then(|| prototype.shape().len().saturating_sub(1));
            Primitive::Replicate.call_axis(Some(&Value::integer(0)), prototype, axis, span)
        }
        (_, Value::Function(_)) => Err(span.domain_error("function elements have no numeric reduction identity")),
        (_, a @ Value::Array(_)) => {
            let data = a.elements().map(|e| identity(operand, &e, span)).collect::<Result<Vec<_>, _>>()?;
            let fill = identity(operand, &a.prototype(), span)?;
            a.layout().collect(data, fill).error_at(span, "invalid identity")
        }
        (Boolean(b), _) => Ok(Value::Number(crate::Number::from_bool(b))),
        (Logical(b), Value::Number(value)) if value.as_bool().is_some() => Ok(Value::Number(crate::Number::from_bool(b))),
        (Infinity(positive), _) => number(if positive { f64::INFINITY } else { f64::NEG_INFINITY }),
        (Number(n), Value::Number(value)) => Ok(Value::Number(value.like(n))),
        (Logical(b), Value::Number(value)) => Ok(Value::Number(value.like(b.into()))),
        (Number(n), _) => number(f64::from(n)),
        (Logical(b), _) => number(f64::from(u8::from(b))),
    }
}

pub(super) fn fold(
    operand: &Function, kind: FoldKind, axis: Option<&Value>, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    if left.is_none() && cx.session.transforming() {
        if right.linear().is_some() || right.dual_tag().is_some() {
            if let Some(folded) = in_turn(operand, kind, axis, right, cx)? { return Ok(folded); }
        }
        if let Some(t) = right.linear() {
            let plus = matches!(operand.as_primitive(), Some(Primitive::Arithmetic(crate::number::Arithmetic::Plus)));
            return if plus { crate::linear::fold(kind, axis, t, cx) } else { Err(cx.span.error(crate::ErrorKind::Unsupported, "∂ can't transpose this fold")) };
        }
        if let Some(result) = fold_dual(operand, kind, axis, right, cx) { return result; }
    }
    let axis = axis.map(|a| crate::primitive::resolve_axes(a, right, cx.span)).transpose()?;
    if !kind.scan {
        if let Some(axes) = axis.as_ref().filter(|a| !a.is_singleton()) {
            if folds_whole(operand, left, right) {
                if left.is_none() { if let Some(folded) = in_turn(operand, kind, Some(axes), right, cx)? { return Ok(folded); } }
                return fold_ravels(operand, kind, axes, left, right, cx);
            }
            let mut reduction = Function::new(FunctionNode::Fold(operand.clone(), kind), cx.span)?;
            if let Some(seed) = left { reduction = before(seed.clone(), reduction, cx.span)?; }
            let ravelled = Function::new(
                FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(reduction), Operand::Function(Function::primitive(Primitive::Ravel))]), cx.span)?;
            // The seed binds first, so the general axis rule doesn't split it along the axes.
            return cell_axes(&ravelled, axes, None, right, cx)?.array(cx.span);
        }
    }
    if let (None, None, Some(p)) = (&axis, left, operand.as_primitive()) {
        if let Some(folded) = crate::fusion::fold(right, p, kind) { return Ok(folded); }
    }
    let result = fold_array(operand, kind, axis.as_ref(), left, right, cx)?;
    if !kind.scan && right.shape().len() == 1 { return Ok(result.at(0)); }
    if kind.scan || right.is_unit() { return Ok(result); }
    let axis = kind.axis(axis.as_ref(), right, cx.span)?;
    let layout = right.layout().axes((0..right.shape().len()).filter(|&a| a != axis));
    result.with_layout(layout).error_at(cx.span, "invalid fold keys")
}

/// Whether a fold by `operand` gives a simple result for each cell of `right`: a pervasive primitive on a simple array, with no seed or an atom seed. Other folds can give arrays or nested results. Folds over several axes, and Rank's rules for whole arrays, then call the fold once for each cell.
fn folds_whole(operand: &Function, left: Option<&Value>, right: &Value) -> bool { right.is_simple() && left.is_none_or(Value::is_atom) && pervasive_primitive(operand, true).is_some() }

/// A fold of a dual, by a rule where its operand has one: `+` folds the primal and the tangent alike, and `×`, `⌈` and `⌊` along the fold's own axis have rules of whole-array calls. `None` leaves the fold to its general path, which calls the operand on the dual's items.
#[cold]
#[inline(never)]
fn fold_dual(operand: &Function, kind: FoldKind, axis: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    use crate::number::{Arithmetic::*, Math::{Ceiling, Floor}};
    let Some(p) = operand.as_primitive() else { return None };
    let (tag, x, tx) = right.dual()?;
    Some(match p {
        Primitive::Arithmetic(Plus) => fold(operand, kind, axis, None, &x, cx)
            .and_then(|r| Ok(crate::derivative::dual(tag, r, fold(operand, kind, axis, None, &tx, cx)?))),
        Primitive::Arithmetic(Times) | Primitive::Math(Ceiling | Floor) if !x.shape().is_empty() => fold_rule(operand, p, kind, axis, &x, &tx, cx).map(|(r, t)| crate::derivative::dual(tag, r, t)),
        _ => return None,
    })
}

/// A fold by `+`, `×`, `⌈` or `⌊` along several axes, as folds along one axis at a time from the last, for an argument that `∂` differentiates. The four operands are associative and commutative. Folding one axis at a time therefore gives the fold of the ravelled cells. `None` for any other fold.
#[cold]
#[inline(never)]
fn in_turn(operand: &Function, kind: FoldKind, axis: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Option<Value>, Error> {
    use crate::number::{Arithmetic::*, Math::{Ceiling, Floor}};
    let Some(axis) = axis.filter(|_| !kind.scan) else { return Ok(None) };
    if !matches!(operand.as_primitive(), Some(Primitive::Arithmetic(Plus | Times) | Primitive::Math(Ceiling | Floor))) { return Ok(None); }
    let axes = crate::primitive::resolve_axes(axis, right, cx.span)?;
    if axes.is_singleton() { return Ok(None); }
    let mut axes = crate::primitive::axes(&axes, right.shape().len(), cx)?;
    axes.sort_unstable_by(|a, b| b.cmp(a));
    let mut folded = right.clone();
    for a in axes { folded = fold(operand, kind, Some(&Value::integer(a as i64)), None, &folded, cx)?; }
    Ok(Some(folded))
}

/// A fold over several axes, as one fold of each cell's ravel. The selected axes move last, in their order, and merge into one axis.
fn fold_ravels(operand: &Function, kind: FoldKind, axes: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let selected = crate::primitive::axes(axes, right.shape().len(), cx)?;
    let others: Vec<usize> = (0..right.shape().len()).filter(|i| !selected.contains(i)).collect();
    let mut shape: Vec<usize> = others.iter().map(|&i| right.shape()[i]).collect();
    shape.push(selected.iter().map(|&i| right.shape()[i]).product());
    let ravels = crate::primitive::reshape_to(&shape, &moved_last(right, &selected, cx)?, cx)?;
    let last = Value::integer(others.len() as i64);
    let folded = fold(operand, kind, Some(&last), left, &ravels, cx)?;
    if others.is_empty() { return Ok(folded); }
    folded.with_layout(right.layout().axes(others.iter().copied())).error_at(cx.span, "invalid fold keys")
}

/// The primal and tangent of a fold or scan by `p`, which is `×`, `⌈` or `⌊`, of `x` with tangent `tx`, through the linear operations that reverse mode transposes. A product's tangent scales each item's tangent by the product of the others and sums: the product of the nonzero items, times the sum of `tx÷x` where no item is zero, or times the zero item's tangent where one is, and 0 where more are. A maximum's or minimum's tangent gathers the tangent of the item that gives it, the first of any ties, as dyadic `⌈` and `⌊` give a tie to their left argument.
fn fold_rule(operand: &Function, p: Primitive, kind: FoldKind, axis: Option<&Value>, x: &Value, tx: &Value, cx: &mut Context<'_>) -> Result<(Value, Value), Error> {
    use crate::{number::Arithmetic::*, primitive::Comparison::{Equal, NotEqual}};
    let call = |p: Primitive, a: &Value, b: &Value, cx: &Context<'_>| p.call(Some(a), b, cx);
    let (times, plus) = (Primitive::Arithmetic(Times), Function::primitive(Primitive::Arithmetic(Plus)));
    let r = fold(operand, kind, axis, None, x, cx)?;
    let t = if matches!(p, Primitive::Arithmetic(Times)) {
        let zero = call(Primitive::Compare(Equal), x, &Value::integer(0), cx)?;
        let ones = call(Primitive::Arithmetic(Plus), x, &zero, cx)?;
        let rest = fold(operand, kind, axis, None, &ones, cx)?;
        let zeros = fold(&plus, kind, axis, None, &zero, cx)?;
        let reciprocal = call(Primitive::Arithmetic(Divide), &call(Primitive::Arithmetic(Minus), &Value::integer(1), &zero, cx)?, &ones, cx)?;
        let none = fold(&plus, kind, axis, None, &call(times, &reciprocal, tx, cx)?, cx)?;
        let one = fold(&plus, kind, axis, None, &call(times, &zero, tx, cx)?, cx)?;
        let none = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &Value::integer(0), cx)?, cx)?, &none, cx)?;
        let one = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &Value::integer(1), cx)?, cx)?, &one, cx)?;
        call(Primitive::Arithmetic(Plus), &none, &one, cx)?
    } else {
        let shape = x.shape();
        let along = kind.axis(axis, x, cx.span)?;
        let lanes = Lanes::new(shape, along).at(cx.span)?;
        let Lanes { outer, len: n, inner } = lanes;
        let booleans = |v: Value| v.as_booleans().map(<[bool]>::to_vec).unwrap_or_else(|| v.elements().map(|e| matches!(e, Value::Number(n) if !n.is_zero())).collect());
        let (offsets, one_per_lane) = if kind.scan {
            // A running maximum or minimum changes only where an item gives a new extreme, whose tangent it then follows. Where it doesn't change, the earlier item keeps it.
            let drop = |d: i64, cx: &Context<'_>| {
                let spec = Value::integers(vec![along + 1], (0..=along).map(|a| if a == along { d } else { 0 }).collect()).error_at(cx.span, "invalid drop")?;
                Primitive::Drop.call(Some(&spec), &r, cx)
            };
            let changed = booleans(call(Primitive::Compare(NotEqual), &drop(1, cx)?, &drop(-1, cx)?, cx)?);
            let mut offsets = Vec::with_capacity(outer * n * inner);
            for o in 0..outer {
                for k in 0..n {
                    for i in 0..inner {
                        let at = lanes.offset(o, k, i);
                        offsets.push(if k == 0 || changed[(o * (n - 1) + k - 1) * inner + i] { at as i64 } else { offsets[at - inner] });
                    }
                }
            }
            (offsets, false)
        } else if let Some(offsets) = (along == kind.default_axis(shape.len())).then(|| crate::fusion::picks(x, p, kind)).flatten() {
            // Windows that slide along the fold's axis give their extremes' positions from their source, with no windows built.
            (offsets, false)
        } else {
            // `r` takes a length-1 axis in place of the folded axis to pair with `x`, unless the fold removed the last axis.
            let back = if along + 1 < shape.len() {
                let mut shape = r.shape().iter().map(|&n| n as i64).collect::<Vec<_>>();
                shape.insert(along, 1);
                Primitive::Shape.call(Some(&Value::integers(vec![shape.len()], shape).error_at(cx.span, "invalid shape")?), &r, cx)?
            } else { r.clone() };
            let hit = booleans(call(Primitive::Compare(Equal), x, &back, cx)?);
            // The first item of each lane that gives its extreme, or the lane's first item where none does, as for NaN.
            let firsts = (0..outer)
                .flat_map(|o| (0..inner).map(move |i| (o, i)))
                .map(|(o, i)| (0..n).map(|k| lanes.offset(o, k, i)).find(|&at| hit[at]).unwrap_or(lanes.offset(o, 0, i)) as i64)
                .collect::<Vec<_>>();
            (firsts, true)
        };
        // Where reverse mode records one item from each lane, the tangent folds the tangent times a mask of the chosen items. Its transpose reads: each item takes its lane's cotangent where it was chosen, and 0 elsewhere. A gather's transpose would scatter.
        if one_per_lane && (tx.linear().is_some() || crate::derivative::active(tx)) {
            let mut chosen = vec![false; x.len()];
            for &at in &offsets { chosen[at as usize] = true; }
            let mask = x.layout().booleans(chosen).error_at(cx.span, "invalid mask")?;
            return Ok((r, fold(&plus, kind, axis, None, &call(times, &mask, tx, cx)?, cx)?));
        }
        let offsets = Value::integers(vec![offsets.len()], offsets).error_at(cx.span, "invalid positions")?.enclose().expect("positions enclose");
        let picked = Primitive::Index.call(Some(&offsets), &Primitive::Ravel.call(None, tx, cx)?, cx)?;
        if r.is_atom() { picked.at(0) } else { picked.relaid(r.layout().clone()).expect("the gathered tangents fill the fold's result") }
    };
    Ok((r, t))
}

fn fold_array(operand: &Function, kind: FoldKind, axis: Option<&Value>, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    // Scan keeps every position, so it keeps every key.
    if kind.scan { return scan(operand, kind, axis, left, right, cx)?.with_layout(right.layout().clone()).error_at(cx.span, "invalid scan result"); }
    if right.is_unit() && axis.is_none() { return match left { Some(seed) => operand.call_array(Some(&right.at(0)), seed, cx), None => Ok(right.at(0)) }; }
    let axis = kind.axis(axis, right, cx.span)?;
    if axis >= right.shape().len() { return Err(cx.span.domain_error("axis is outside array rank")); }
    let traversal = Lanes::new(right.shape(), axis).at(cx.span)?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    let size = generated_len(&shape).at(cx.span)?;
    if size == 0 { return Value::empty(shape, right.prototype()).error_at(cx.span, "invalid empty fold"); }
    if traversal.len == 0 {
        let item = match left { Some(seed) => seed.clone(), None => identity(operand, &right.prototype(), cx)? };
        return Value::new(shape, vec![item; size]).error_at(cx.span, "invalid identity result");
    }
    let active = crate::derivative::active_in(cx, right);
    if let (None, Some(p), false) = (left, operand.as_primitive(), active) {
        use crate::number::Arithmetic::{Plus, Times};
        // Float sums and products take the kernel for any number of items, because they fold in any order.
        if traversal.len >= 2 || (matches!(right.as_items(), crate::array::Items::Compact(rustymath::buffer::View::Floats(_))) && matches!(p, Primitive::Arithmetic(Plus | Times))) {
            if let Some(result) = crate::pervasive::fold(p, right, axis) { return Ok(result); }
        }
        if let Primitive::Arithmetic(op) = p { if right.all_numbers() { return number_lanes(op, false, right, None, &traversal, shape, cx); } }
    }
    let data = fold_items(&traversal, left, |offset| right.at(offset), |x, y| operand.call_array(Some(x), y, cx))?;
    Value::new(shape, data).error_at(cx.span, "invalid fold result")
}

/// A fold or scan by `op` of plain numbers, with an optional `seed`: the general path for an arithmetic primitive when every item is a number.
fn number_lanes(
    op: crate::number::Arithmetic, scan: bool, right: &Value, seed: Option<&Value>, axis: &Lanes, shape: Vec<usize>, cx: &Context<'_>) -> Result<Value, Error> {
    let number = |e: Value| { let Value::Number(n) = e else { unreachable!() }; n };
    let (item, seed) = (|i| number(right.at(i)), seed.cloned().map(number));
    let (p, t) = (Primitive::Arithmetic(op), Numeric::current());
    let step = |x: &crate::Number, y: &crate::Number| { cx.check()?; p.number(Some(x), y, t).domain_at(cx.span) };
    let data = if scan { scan_items(axis, seed, item, step)? } else { fold_items(axis, seed.as_ref(), item, step)? };
    Value::new(shape, data.into_iter().map(Value::Number).collect()).error_at(cx.span, "invalid numeric result")
}

/// Each lane of `axis` reduced from the right, starting from `seed` or else the lane's last item: `apply(item, result)`.
fn fold_items<T: Clone>(axis: &Lanes, seed: Option<&T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = Vec::with_capacity(axis.outer * axis.inner);
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| item(axis.offset(i, j, k));
            let (mut result, end) = match seed { Some(seed) => (seed.clone(), axis.len), None => (item(axis.len - 1), axis.len - 1) };
            for j in (0..end).rev() { result = apply(&item(j), &result)?; }
            data.push(result);
        }
    }
    Ok(data)
}

fn scan_items<T: Clone>(axis: &Lanes, seed: Option<T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = vec![item(0); axis.outer * axis.len * axis.inner];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let first = axis.offset(i, 0, k);
            let (mut value, start) = match seed.clone() {
                Some(value) => (value, 0),
                None => {
                    let value = item(first);
                    data[first] = value.clone();
                    (value, 1)
                }
            };
            for j in start..axis.len {
                let index = axis.offset(i, j, k);
                value = apply(&value, &item(index))?;
                data[index] = value.clone();
            }
        }
    }
    Ok(data)
}

fn scan_axis(kind: FoldKind, axis: Option<&Value>, right: &Value, span: &Span) -> Result<Lanes, Error> {
    if right.is_unit() && axis.is_none() { return Ok(Lanes { outer: 1, len: 1, inner: 1 }); }
    let index = kind.axis(axis, right, span)?;
    Lanes::new(right.shape(), index).at(span)
}

pub(super) fn scan(
    operand: &Function, kind: FoldKind, axis: Option<&Value>, seed: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let axis = scan_axis(kind, axis, right, cx.span)?;
    if right.is_atom() { return match seed { Some(seed) => operand.call_array(Some(seed), right, cx), None => Ok(right.clone()) }; }
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    // Kernels that read numbers directly leave a dual to the general path, which calls the operand through dispatch.
    let active = crate::derivative::active_in(cx, right);
    if let (Some(p), false) = (operand.as_primitive(), active) {
        if let Some(result) = crate::pervasive::scan(p, right, seed, &axis) { return Ok(result); }
    }
    if let (Some(Primitive::Arithmetic(op)), false) = (operand.as_primitive(), active) {
        if right.all_numbers() && seed.is_none_or(|a| matches!(a, Value::Number(_))) {
            return number_lanes(op, true, right, seed, &axis, right.shape().to_vec(), cx);
        }
    }
    let data = scan_items(&axis, seed.cloned(), |i| right.at(i), |x, y| operand.call_array(Some(x), y, cx))?;
    Value::new(right.shape().to_vec(), data).error_at(cx.span, "invalid scan result")
}
