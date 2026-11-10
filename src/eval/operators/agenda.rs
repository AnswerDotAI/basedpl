//! Agenda (`⍚`) and Under (`@`).
use super::*;

/// The case that the unit `index` picks from `len` cases (`cases_picked`). A selector that isn't a number is DOMAIN.
pub(in crate::eval) fn agenda_index(index: &Value, len: usize, span: &Span) -> Result<usize, Error> {
    let Plain::Number(_) = index.observed()? else { return Err(Error::domain("agenda index must be numeric")) };
    let [k] = crate::primitive::positions(&cases_picked(index, len, span)?, len)?[..] else { unreachable!("a unit picks one case") };
    Ok(k)
}

/// The case that each value of `selector` picks from `len` cases: the value floored, then wrapped into `0..len` (`len|⌊selector`), so every number picks a case and a negative one counts back from the end. Booleans already pick their own case where there are at least two.
fn cases_picked(selector: &Value, len: usize, span: &Span) -> Result<Value, Error> {
    use crate::number::Math::{Floor, Magnitude};
    if selector.dtype() == Some(rustymath::buffer::DType::Boolean) && len >= 2 { return Ok(selector.clone()); }
    let floored = Primitive::Math(Floor).call(None, selector, span)?;
    Primitive::Math(Magnitude).call(Some(&Value::integer(len as i64)), &floored, span)
}

/// `selector⍚cases`. A unit selector picks one case, which takes the whole arguments. Any other selector picks a case for each cell (`Cells::chosen`), or, where a select can run both cases on every item, selects their results item by item (`selected`).
pub(super) fn agenda<L: Argument, R: Argument>(
    operands: &[Operand; 2], left: Option<L>, right: R, text: impl FnOnce() -> String, span: &Span) -> Result<Bound, Error> {
    let [selector, Operand::Value(fs)] = operands else { unreachable!() };
    let (x, y) = (left.as_ref().map(L::value), right.value());
    let index = computed(selector, x, y, span)?;
    if !index.is_unit() {
        let cases: Vec<_> = fs.elements().map(|f| match f { Value::Function(f) => f, _ => unreachable!() }).collect();
        if let [no, yes, ..] = &cases[..] {
            if let Some(result) = selected(&index, [Some(no), Some(yes)], x, y, text, span)? { return Ok(Bound::from(result)); }
        }
        return Ok(Bound::from(Cells::new(&index, x, y)?.chosen(&index, &cases, x, y, span)?));
    }
    let Value::Function(f) = fs.at(agenda_index(&index, fs.len(), span)?) else { unreachable!() };
    f.call_with(left, right, span)
}

/// An operand that gives an array: the array itself, or a function's result on the arguments, as `⍤`, `⍠`, `⌺` and `⍚` take them.
pub(super) fn computed(operand: &Operand, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    match operand { Operand::Value(a) => Ok(a.clone()), Operand::Function(g) => g.call_array(left, right, span) }
}

/// `f@g`: `f` applied to the selection that `g` makes from the argument, then written back in its place. An array `f` replaces the selection. An array `g` lists positions along the leading axis, as `[g]⌷` reads them, and a selection of no items leaves the argument as it is, without calling `f`. `[p]` runs `f` on the cells where `p` of the argument is 1 (`masked`). With positions or `[p]`, a left argument is laid over the right argument (`Laid`), and a unit goes whole to `f`.
pub(super) fn under(operands: &[Operand; 2], left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span) -> Result<Bound, Error> {
    let [Operand::Function(f), g] = operands else { unreachable!() };
    let positions = match g {
        Operand::Function(g) => return under_function(f, g, left, right, span),
        Operand::Value(a) => match predicate(a) {
            Some(p) => return Ok(Bound::from(masked(f, &p.call_array(None, right, span)?, left, right, text, span)?)), None => a
        },
    };
    let selection = crate::primitive::at_indices(right, positions, span)?;
    if selection.targets.len() == 0 { return Ok(Bound::from(right.clone())); }
    let x = match left {
        Some(x) if !x.is_unit() => {
            let x = Laid::new(x, &right.layout().axes(0..1))?.aligned()?;
            Some(crate::primitive::at_indices(&x, positions, span)?.read(&x, None)?)
        }
        _ => None,
    };
    let values = f.call_with(x.map(Cow::Owned).or(left.map(Cow::Borrowed)), selection.read(right, None)?, span)?.array(span)?;
    Ok(Bound::from(selection.write(right, &values, span)?))
}

/// The function of `[p]`, a one-item vector that holds a function.
fn predicate(a: &Value) -> Option<Function> { if a.shape() != [1] { return None; } match a.at(0) { Value::Function(p) => Some(p), _ => None } }

/// `f@[p]`, where `p` gives the mask `selector`: `f` runs once on the cells whose mask isn't 0 (`fusion::truths`, `Cells::marked`), and its results go back in their places under assignment's rule. Every other cell keeps its value, as does one whose key the mask lacks. A unit mask marks the whole argument or none of it. Where a select can run `f` on every item, it does (`selected`), and `•explain` names the call with `text`.
fn masked(f: &Function, selector: &Value, left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span) -> Result<Value, Error> {
    let selector = &*crate::fusion::truths(selector, span)?;
    if selector.shape().is_empty() {
        let [k] = crate::primitive::positions(selector, 2)?[..] else { unreachable!("a unit has one position") };
        if k != 1 { return Ok(right.clone()); }
        let every = Selection { frame: ResultFrame::Array(right.layout().clone()), targets: Targets::Offsets((0..right.len()).collect()) };
        return every.write(right, &f.call_array(left, right, span)?, span);
    }
    if let Some(result) = selected(selector, [None, Some(f)], left, right, text, span)? { return Ok(result); }
    let cells = Cells::new(selector, left, right)?;
    let rows = cells.marked(selector)?;
    if rows.is_empty() { return Ok(right.clone()); }
    let selection = cells.framed.selection(&rows)?;
    let values = cells.case(f, &rows, left, right, span)?;
    selection.write(right, &values, span)
}

/// The costs, in copies per item, that decide between a select and a gather of each case's cells (`Cells`). A gather pays `GATHERED` for every item, and `PICKED` plus the case's cost for each item that picks a case that computes. A select pays `CHOSEN` for every item, or `FUSED` where fusion runs it with the comparison that gives its mask, plus the cost of every case that computes. `GATHERED`, `PICKED` and `CHOSEN` come from a fit on 19 October 2026 on this Mac, an Apple M5 Max, to `-@[p] s` and `*@[p] s` on 65,536 floats with 10% to 90% of items picked. They put the crossover at about 22% picked for `-` and 55% for `*`. `FUSED` comes from `-@[p<]x` and `*@[p<]x` on a million floats on 20 October 2026, where a fused select cost 2.0 copies plus its kernel for `-`, and 2.3 for `*`. It puts the crossover at about 10% picked for `-` and 47% for `*`. CI's timings can refine all four.
const GATHERED: f64 = 2.3;
const PICKED: f64 = 7.3;
const CHOSEN: f64 = 3.1;
const FUSED: f64 = 2.1;
/// The items of a pending mask that `selected` reads to estimate the share of items that pick each case. Fusion computes only those items (`fusion::walk_through`). The rest of the mask stays pending for the select.
const SAMPLE: usize = 256;

/// `cases[1]` on the items where the Boolean `mask` is 1 and `cases[0]` where it's 0, each run on every item of `right` and then chosen item by item (`fusion::select`), in place of gathering each case's cells (`Cells`). A case of `None` keeps `right`'s items, as `@[p]` keeps the items it doesn't select. A case then also runs on items that didn't pick it, which can't show where it never fails and gives the argument's kind (`select_cost`). `None` leaves the call to `Cells`: for a left argument, an empty argument, a mask and cases that `plan::select` refuses, such as a mask that isn't Booleans with the argument's layout, an argument that isn't floats of 32 or 64 bits, an argument that `∂` tracks, whose gradient a select could make NaN where a case's slope is infinite on an item that didn't pick it, or a case that a select can't run. A select also has to cost less than the gather, by its cases' costs and the share of items that pick each (`GATHERED`). The share is counted from a stored mask, and estimated from a sample of a comparison that fusion runs inside the select (`share`). `•explain` reports a select as a fast combination, named by `text`.
fn selected(
    mask: &Value, cases: [Option<&Function>; 2], left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span
) -> Result<Option<Value>, Error> {
    use rustymath::buffer::{DType, FloatWidth};
    let Some(DType::Float(width @ (FloatWidth::F32 | FloatWidth::F64))) = right.dtype() else { return Ok(None) };
    if left.is_some() || right.is_empty() || **mask.layout() != **right.layout() || crate::transform::carried(left, right) { return Ok(None); }
    let [Some(no), Some(yes)] = cases.map(|f| f.map_or(Some(0), |f| select_cost(f, width))) else { return Ok(None) };
    // Each case holds a constant's kind and shape, or else the argument's, which every other case keeps (`select_cost`).
    let [no_case, yes_case] = cases.map(|f| crate::plan::Operand::Held(f.and_then(Function::as_constant).unwrap_or(right)));
    let Ok(plan) = crate::plan::select(&crate::plan::Operand::Held(mask), &yes_case, &no_case) else { return Ok(None) };
    let keeps = |f: Option<&Function>| f.is_none_or(|f| matches!(f.operation(), Some(Operation::Primitive(Primitive::Identity(_)))));
    let fused = crate::fusion::defers_select(mask, right.layout());
    let ones = share(mask, fused);
    let picked = |f: Option<&Function>, share: f64, copies: u32| if keeps(f) { 0.0 } else { share * (PICKED + f64::from(copies)) };
    // `share` counts, and so computes, a mask of at most `SAMPLE` items, which can then no longer run inside the select.
    let chosen = if fused && mask.len() > SAMPLE { FUSED } else { CHOSEN };
    if chosen + f64::from(no + yes) >= GATHERED + picked(cases[0], 1.0 - ones, no) + picked(cases[1], ones, yes) { return Ok(None); }
    let items = |f: Option<&Function>| -> Result<Value, Error> {
        let Some(f) = f.filter(|_| !keeps(f)) else { return Ok(right.clone()) };
        if let Some(a) = f.as_constant() { return Ok(a.clone()); }
        f.call_array(None, right, span)
    };
    let [no, yes] = cases;
    let (y, x) = (items(no)?, items(yes)?);
    if crate::events::running() { crate::explain::fast(crate::events::routes::MaskedSelect, text()); }
    Ok(Some(crate::fusion::select(mask, &x, &y, right.layout(), &plan)))
}

/// The share of `mask`'s items that are 1, counted, or estimated from `SAMPLE` items where `sampled`. An odd stride keeps the sample from lining up with a mask that repeats with a period of a power of two.
fn share(mask: &Value, sampled: bool) -> f64 {
    let n = mask.len();
    let k = if sampled { SAMPLE.min(n) } else { n };
    let sample = (k < n).then(|| {
        crate::explain::uncounted(|| {
            crate::fusion::walked(mask, crate::array::Layout::from(vec![k]), 0, &[rustymath::gather::Steps::along(k, (n / k - 1) | 1)], None)
        }).expect("a sample fits its layout")
    });
    let crate::array::Items::Compact(rustymath::buffer::View::Booleans(bits)) = sample.as_ref().unwrap_or(mask).as_items() else {
        unreachable!("a Boolean mask is compact")
    };
    bits.iter().filter(|&&b| b).count() as f64 / k as f64
}

/// What a select pays to run `f` on every item of an array of floats of `width`, in copies per item (`Facts::cost`), or `None` where it can't run `f`. A select can run `f` where it never fails, and gives one float of that width for each item, from that item alone. That holds for `⊢`, for a constant float of exactly `width`, and for trains built from those and from primitives whose float kernel never fails and gives floats (`pervasive::kernel`). A constant beside such a primitive can be narrower, because the primitive's result takes the argument's width. Any other constant would make the result's kind depend on which items pick the case.
fn select_cost(f: &Function, width: rustymath::buffer::FloatWidth) -> Option<u32> {
    use rustymath::buffer::DType;
    let real = |a: &Value| match a { Value::Number(n) if !n.is_exact() && n.as_bool().is_none() && n.as_float().is_some() => a.float_width(), _ => None };
    let numeric = Numeric::current();
    let kernel = |p: Primitive, dyadic: bool| {
        let copies = u32::from(p.facts(dyadic).cost);
        crate::pervasive::kernel_at(p, None, DType::Float(width), 1 + usize::from(dyadic), numeric)
            .is_some_and(|k| crate::pervasive::settles(&k) && matches!(k.result_dtype(), DType::Float(_))).then_some(copies)
    };
    let dyad = |g: &Function| g.dyadic_primitive().and_then(|p| kernel(p, true));
    if let Some(a) = f.as_constant() { return (real(a) == Some(width)).then_some(0); }
    if let Some((parts, bound)) = f.called_parts(false) {
        if bound.is_some_and(|a| !real(a).is_some_and(|w| w <= width)) { return None; }
        return parts.iter().try_fold(0, |cost, &(g, dyadic)| Some(cost + if dyadic { dyad(g)? } else { select_cost(g, width)? }));
    }
    match f.node() { FunctionNode::Primitive(Primitive::Identity(_)) => Some(0), FunctionNode::Primitive(p) => kernel(*p, false), _ => None }
}

/// A case's result, classified once: one cell for each cell that picks the case, or one item that fills them all, as `@` fills a selection.
enum Made { Cells(Value), Fill(Value) }

impl Made {
    /// `values` from a case whose cells have the list frame `rows`. A singleton fills; any other result aligns by key and needs one result per row.
    fn new(values: Value, rows: &crate::array::Layout) -> Result<Self, Error> {
        if values.is_singleton() { return Ok(Self::Fill(values)); }
        let values = crate::keyed::reorder(&values, &rows.all_keys(), true).described("case result does not supply selected keys")?;
        if values.shape().first() == rows.shape().first() { return Ok(Self::Cells(values)); }
        Err(Error::new(Kind::Length, "a case needs one result for each cell that picks it"))
    }
    fn value(&self) -> &Value { match self { Self::Cells(v) | Self::Fill(v) => v } }
}

/// The cells of `right` along the axes that a selector's axes pair with, for `⍚` with a selector for each cell and for `@[p]`. The selector's axes pair with the argument's by name and then from the front. The selector, and a left argument that isn't a unit, are each laid over the frame as agreement lays them (`lookup`, `Laid`). Cells are read, and written, where they lie (`Framed`): no axis of either argument moves.
struct Cells { framed: Framed, axes: Vec<usize>, lookup: Mapping, x: Option<Laid>, len: usize }

impl Cells {
    fn new(selector: &Value, left: Option<&Value>, right: &Value) -> Result<Self, Error> {
        let axes = rustymath::agreement::Agreement::paired_axes(right.layout(), selector.layout(), true);
        if axes.iter().any(|&a| a >= right.shape().len()) { return Err(Error::new(Kind::Rank, "the selector has an axis that the argument lacks")); }
        let framed = Framed::new(right.layout(), &axes)?;
        let frame = framed.frame();
        let len = frame.count();
        let lookup = lookup(selector.layout(), &frame, len, "the selector doesn't agree with the argument")?;
        let x = left.filter(|x| !x.is_unit()).map(|x| Laid::new(x, &frame)).transpose()?;
        Ok(Self { framed, axes, lookup, x, len })
    }

    /// The frame positions that `selector` marks 1, in order. A Boolean mask that lies in frame order goes straight to `⍸`'s kernel. Any other mask decodes as `picked` decodes it.
    fn marked(&self, selector: &Value) -> Result<Vec<u32>, Error> {
        if let (Mapping::Linear(1), crate::array::Items::Compact(rustymath::buffer::View::Booleans(mask))) = (&self.lookup, selector.as_items()) {
            let mut rows = Vec::new();
            rustymath::gather::positions(&mut rows, mask, mask.iter().map(|&b| usize::from(b)).sum(), |j| j as u32);
            return Ok(rows);
        }
        Ok(self.picked(selector, 2)?.each().nth(1).unwrap_or_default().to_vec())
    }

    /// The frame positions grouped by the case of `cases` that each picks (`picked`).
    fn picked(&self, selector: &Value, cases: usize) -> Result<rustymath::search::Groups, Error> {
        picked(selector, (!matches!(self.lookup, Mapping::Linear(1))).then_some(&self.lookup), self.len, cases)
    }

    /// The left argument's cells at frame positions `rows`, or `None` where the left argument is a unit or absent.
    fn left_cells(&self, rows: &[u32]) -> Result<Option<Value>, Error> { self.x.as_ref().map(|x| x.cells(rows)).transpose() }

    /// The result of case `f` on the cells of `right` at frame positions `rows`. `f` gets the left argument's cells at the same positions, or `left` whole where it's a unit. A constant case doesn't look at its cells.
    fn case(&self, f: &Function, rows: &[u32], left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        if let Some(a) = f.as_constant() { return Ok(a.clone()); }
        let xs = self.left_cells(rows)?;
        f.call_with(xs.map(Cow::Owned).or(left.map(Cow::Borrowed)), self.framed.read(right, rows)?, span)?.array(span)
    }

    /// `⍚` with a selector for each cell. Each case runs once, on the list of all the cells that pick it (`case`), and gives one result for each of them or one item for all of them (`Made`). Each cell picks the case that `cases_picked` gives its selector value. A case that no cell picks doesn't run. Numbers keep their kinds. A case of `⊢` doesn't run: when every other result has the argument's cells, the result starts from the argument, so `⊢`'s cells stay where they are. Otherwise `⊢`'s cells join the other results.
    fn chosen(&self, selector: &Value, cases: &[Function], left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let selector = &cases_picked(selector, cases.len(), span)?;
        if self.len == 0 { return self.no_cells(selector, cases, left, right, span); }
        let groups = self.picked(selector, cases.len())?;
        let picked: Vec<&[u32]> = groups.each().collect();
        let keeps = |k: usize| matches!(cases[k].operation(), Some(Operation::Primitive(Primitive::Identity(false))));
        let mut made = Vec::new();
        for (k, f) in cases.iter().enumerate() {
            let rows = picked[k];
            if rows.is_empty() || keeps(k) { continue; }
            let values = self.case(f, rows, left, right, span)?;
            let frame = listed(&self.framed.frame(), rows, &Default::default())?;
            made.push((k, Made::new(values, &frame)?));
        }
        let kept: Vec<usize> = (0..cases.len()).filter(|&k| keeps(k) && !picked[k].is_empty()).collect();
        // The first case that some cell picks gives the fill. A kept case's cells are the argument's.
        let fill = match (kept.first(), made.first()) {
            (Some(&k), first) if first.is_none_or(|(j, _)| k < *j) => right.prototype(), (_, Some((_, m))) => m.value().prototype(), _ => right.prototype()
        };
        let cell = self.framed.cells();
        let start = (!kept.is_empty() && made.iter().all(|(_, m)| matches!(m, Made::Fill(_)) || m.value().shape()[1..] == *cell.shape())).then_some(right);
        if start.is_none() {
            for &k in &kept { made.push((k, Made::Cells(self.framed.read(right, picked[k])?))); }
            made.sort_by_key(|&(k, _)| k);
        }
        self.assembled(&picked, picked.get(cases.len()).copied().unwrap_or_default(), &made, fill, start)
    }

    /// The results of the cases as one array, laid out as `placed` gives it. `none` lists the cells that pick no case, which get `fill`. A fill result fills each cell of its case. Results whose cells differ in shape are padded, as Rank pads them, and a fill result spreads across the shape that the others pad to. `start`, where a case of `⊢` keeps its cells and every result has the argument's cells, is the argument, which the result starts from. Numbers keep their kinds.
    fn assembled(&self, picked: &[&[u32]], none: &[u32], made: &[(usize, Made)], fill: Value, start: Option<&Value>) -> Result<Value, Error> {
        let cells = || {
            made.iter().filter_map(|(k, m)| match m { Made::Cells(v) => Some((*k, v)), Made::Fill(_) => None })
        };
        let mut layouts: Vec<_> = cells().map(|(_, v)| v.layout().axes(1..v.shape().len())).collect();
        if start.is_some() || layouts.is_empty() { layouts.push(self.framed.cells()); }
        let cell = crate::array::Layout::assembled(layouts.iter()).described("invalid result cells")?;
        let shape = cell.shape();
        if cells().any(|(_, v)| v.shape()[1..] != *shape) {
            let mut items = vec![fill.clone(); self.len];
            for (k, m) in made {
                let rows = picked[*k];
                match m {
                    Made::Cells(v) => {
                        let cells = v.cells(v.shape().len() - 1).and_then(|c| c.framed()).described("invalid result")?;
                        for (j, &i) in rows.iter().enumerate() { items[i as usize] = cells.at(j); }
                    }
                    Made::Fill(v) => {
                        let n = cell.count();
                        let mut spread = Gather::items(n);
                        spread.fill(&v.at(0), n);
                        let item = spread.finish(cell.clone(), fill.clone()).described("invalid result")?;
                        for &i in rows { items[i as usize] = item.clone(); }
                    }
                }
            }
            // The case cells give the cells' layout. The scalar fills at cells that pick no case carry none.
            let result =
                Value::assemble_layout(&self.framed.frame(), &items, &fill, crate::array::Widening::Items, None, Some(&cell)).described("invalid result")?;
            let (_, positions) = placed(&self.framed, &self.axes, &cell)?;
            if rustymath::gather::counts_up(&positions) { return Ok(result); }
            return crate::primitive::transpose_axes(fronted(&self.axes, self.framed.layout.shape().len()), &result);
        }
        let (layout, positions) = placed(&self.framed, &self.axes, &cell)?;
        let target = Framed::new(&layout, &positions)?;
        let width = cell.count();
        let mut data = Gather::items(self.len * width);
        // Where every cell picks a case, the case that fills the most cells with one item fills the whole array first.
        let base = if none.is_empty() && start.is_none() {
            made.iter().filter(|(_, m)| matches!(m, Made::Fill(_))).max_by_key(|(k, _)| picked[*k].len()).map(|(k, _)| *k)
        } else { None };
        match start {
            Some(argument) => data.walk(argument, 0, &[rustymath::gather::Steps::Stride { start: 0, len: argument.len(), step: 1 }], None),
            None => data.fill(&made.iter().find(|(k, _)| Some(*k) == base).map_or_else(|| fill.clone(), |(_, m)| m.value().at(0)), self.len * width),
        }
        let mut write = |rows: &[u32], values: &Value| data.scatter(target.targets(rows).writes().expect("cells' targets are offsets or a walk"), values);
        for (k, m) in made.iter().filter(|(k, _)| Some(*k) != base) { write(picked[*k], m.value()) }
        if start.is_some() && !none.is_empty() { write(none, &fill) }
        data.finish(layout, fill).described("invalid result")
    }

    /// `chosen` for a frame with no cells. As Each calls its operand on prototypes, the case that the selector's fill picks runs on the empty list of cells (`on_prototypes`). The result is empty, with that case's cells, laid out as `placed` gives it.
    fn no_cells(&self, selector: &Value, cases: &[Function], left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let k = match crate::primitive::positions(&selector.prototype(), cases.len())?[..] {
            [k] if k != rustymath::gather::Steps::FILL => k, _ => return Err(Error::domain("the selector's fill picks no case"))
        };
        let xs = self.left_cells(&[])?;
        let result = on_prototypes(&cases[k], xs.as_ref().or(left), &self.framed.read(right, &[])?, span)?.array(span)?;
        let cell = if result.is_singleton() { self.framed.cells() }
        else if result.shape().first() == Some(&0) { result.layout().axes(1..result.shape().len()) }
        else { Default::default() };
        placed(&self.framed, &self.axes, &cell)?.0.collect(Vec::new(), result.prototype()).described("invalid result")
    }
}


/// The cells of an array along its frame axes. `layout` and `steps` put the frame's axes first and then a cell's, as `⍉` would, but no item moves. The first `rank` steps reach each cell's start; the rest read its items.
struct Framed { layout: crate::array::Layout, rank: usize, steps: Vec<rustymath::gather::Steps> }

impl Framed {
    fn new(layout: &crate::array::Layout, frame: &[usize]) -> Result<Self, Error> {
        let plan = rustymath::structure::transpose(layout, &rustymath::search::inverse_permutation(fronted(frame, layout.shape().len()).into_iter()))?;
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
            [Steps::Stride { start, step, .. }] => {
                rows.iter().map(|&j| lookup(j as usize).map(|j| start.wrapping_add_signed(step * j as isize)).unwrap_or(Steps::FILL)).collect()
            }
            ref steps => rows.iter().map(|&j| lookup(j as usize).and_then(|j| Steps::reach(0, steps, j)).unwrap_or(Steps::FILL)).collect(),
        }
    }
    /// The targets of the cells at frame positions `rows`, cell by cell. Single items are their offsets, which are the positions themselves where the cells lie in frame order. Wider cells are a walk, a table of the cells' starts followed by the cells' own steps, which reads and writes cells whose items lie a stride apart, as columns' do, in tiles.
    fn targets(&self, rows: &[u32]) -> Targets { self.at_starts(self.starts(rows, Some)) }
    /// The cells at `starts`, with a fill start reading a whole cell of fills. Transpose's cell steps start at 0, so singleton cells need only their starts.
    fn at_starts(&self, starts: Vec<usize>) -> Targets {
        let steps = &self.steps[self.rank..];
        if steps.iter().all(|s| s.len() == 1) { return Targets::Offsets(starts); }
        let starts = rustymath::gather::Steps::table(starts);
        Targets::Walk { base: 0, steps: std::iter::once(starts).chain(steps.iter().cloned()).collect() }
    }
    /// The selection of the cells at frame positions `rows`, as a list (`listed`).
    fn selection(&self, rows: &[u32]) -> Result<Selection, Error> {
        Ok(Selection { frame: ResultFrame::Array(listed(&self.frame(), rows, &self.cells())?), targets: self.targets(rows) })
    }
    /// The cells at frame positions `rows` of `array`, as a list (`listed`).
    fn read(&self, array: &Value, rows: &[u32]) -> Result<Value, Error> { self.selection(rows)?.read(array, None) }
}

/// The layout of the cells at positions `rows` of `frame`, as a list of cells laid out by `cell`. A frame of one axis keeps its name and the keys at `rows`.
fn listed(frame: &crate::array::Layout, rows: &[u32], cell: &crate::array::Layout) -> Result<crate::array::Layout, Error> {
    let list = if frame.shape().len() == 1 { frame.select(0, rows.iter().map(|&j| Some(j as usize))).described("invalid selection")? }
    else { crate::array::Layout::from(vec![rows.len()]) };
    Ok(list.concat(cell))
}

/// The `len` cells of the frame grouped by the case of `cases` that each picks. `mapping` reads each cell's value from `selector`, and `None` reads them in place, where a Boolean is its own case number. A NaN, or a key that the selector lacks, picks no case, and those cells form a group after the cases.
fn picked(selector: &Value, mapping: Option<&Mapping>, len: usize, cases: usize) -> Result<rustymath::search::Groups, Error> {
    use {crate::search::grouped_by, rustymath::gather::Steps};
    match (mapping, selector.as_items()) {
        (None, crate::array::Items::Compact(rustymath::buffer::View::Booleans(mask))) if cases > 1 => grouped_by(mask, cases, usize::from),
        _ => {
            let values = crate::primitive::positions(selector, cases)?;
            let keys = match mapping { None => values, Some(m) => (0..len).map(|j| m.get(j).map_or(Steps::FILL, |i| values[i])).collect() };
            grouped_by(&keys, cases + usize::from(keys.contains(&Steps::FILL)), |p| p.min(cases))
        }
    }
}

/// The map from each of the `len` positions of `frame` to a position of `input`, whose axes lie along the frame's in order, as agreement pairs them. An axis of length 1 stretches across the frame. Where both axes have keys, each position looks its key up (`Mapping::new`): a key that `input` lacks gives no position, and a key that only `input` has is never read. Any other axis needs the frame's length, or the call is a LENGTH ERROR with the message `mismatch`.
fn lookup(input: &crate::array::Layout, frame: &crate::array::Layout, len: usize, mismatch: &str) -> Result<Mapping, Error> {
    let rank = frame.shape().len();
    if (0..rank).any(|a| !(input.keys(a).is_some() && frame.keys(a).is_some()) && ![frame.shape()[a], 1].contains(&input.shape()[a])) {
        return Err(Error::new(Kind::Length, mismatch));
    }
    Ok(Mapping::new(input, &(0..rank).collect::<Vec<_>>(), frame, len))
}

/// A left argument laid over a frame. Its axes pair with the frame's by name and then from the front, and each frame position reads the cell that `lookup` maps it to. A position whose key the argument lacks reads a cell of fills.
struct Laid { value: Value, framed: Framed, lookup: Mapping, frame: crate::array::Layout }

impl Laid {
    fn new(x: &Value, frame: &crate::array::Layout) -> Result<Self, Error> {
        const MISMATCH: &str = "the left argument doesn't line up with the right argument";
        let axes = rustymath::agreement::Agreement::paired_axes(x.layout(), frame, true);
        if axes.iter().any(|&a| a >= x.shape().len()) { return Err(Error::new(Kind::Length, MISMATCH)); }
        let framed = Framed::new(x.layout(), &axes)?;
        let lookup = lookup(&framed.frame(), frame, frame.count(), MISMATCH)?;
        Ok(Self { value: x.clone(), framed, lookup, frame: frame.clone() })
    }

    /// The cells for frame positions `rows`, as a list that keeps the frame's keys at `rows` (`listed`).
    fn cells(&self, rows: &[u32]) -> Result<Value, Error> {
        let starts = self.framed.starts(rows, |j| self.lookup.get(j));
        let layout = listed(&self.frame, rows, &self.framed.cells())?;
        Selection { frame: ResultFrame::Array(layout), targets: self.framed.at_starts(starts) }.read(&self.value, None)
    }

    /// The argument laid over the whole frame: its cells in frame order, with the frame's axes first.
    fn aligned(&self) -> Result<Value, Error> { let len = self.frame.count(); self.cells(&(0..len as u32).collect::<Vec<_>>()) }
}

/// The layout of `Cells::chosen`'s result: the frame of `framed`, whose axes are `axes` of the argument, with cells laid out by `cell`. Where the cells have the rank of the argument's cells, the frame's axes keep their places in the argument. Otherwise they come first, as a frame does in Rank. Also gives the positions of the frame's axes in the result.
fn placed(framed: &Framed, axes: &[usize], cell: &crate::array::Layout) -> Result<(crate::array::Layout, Vec<usize>), Error> {
    let first = framed.frame().concat(cell);
    let n = framed.layout.shape().len();
    if cell.shape().len() != n - axes.len() { return Ok((first, (0..axes.len()).collect())); }
    let plan = rustymath::structure::transpose(&first, &fronted(axes, n))?;
    Ok((plan.layout.into(), axes.to_vec()))
}

/// The order of the axes of an array of rank `n` with the axes `first` moved to the front, in their order, and the others after them, in theirs.
fn fronted(first: &[usize], n: usize) -> Vec<usize> { first.iter().copied().chain((0..n).filter(|a| !first.contains(a))).collect() }

/// `f@g` for a function `g`: `f⍥g`, with the result written back. A `g` that selects writes it back where it took it from. Any other `g` is undone by its inverse.
fn under_function(f: &Function, g: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let (level, selected) = session().with_level(crate::transform::Kind::Label, |level| {
        (level, crate::selection::labelled(right, level).and_then(|(labels, y)| Ok((labels, g.call_with(None::<&Value>, y, span)?.array(span)?))))
    });
    let (mut labels, selected) = selected?;
    let read = labels.read(&selected, level);
    let x = left.map(|x| g.call_array(None, x, span)).transpose()?;
    let result = f.call_with(x, crate::selection::strip(&selected, level), span)?.array(span)?;
    match read {
        Some(read) => {
            let (selection, values) = labels.replacements(read, &result)?;
            Ok(Bound::from(selection.write(right, &values, span)?))
        }
        None if g.discards(false) => Err(Error::domain("Under can't use the inverse of a function that drops items")),
        None => g.inverse(span)?.call_with(None::<&Value>, result, span),
    }
}
