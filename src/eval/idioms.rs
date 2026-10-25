//! Idioms: combinations of functions that have a faster algorithm than calling each function in turn. `Function::new` recognises an idiom from a function's structure when it builds the function, and keeps it beside the node. `dispatch` tries the idiom before the general path, and Key tries its idioms inside `key`, which shares its validation and classes with the general path. The idiom checks its arguments on every call, and gives `None` where they don't suit it, which sends the call down the general path.
//!
//! Only functions get idioms: the trains, compositions and operator applications that build one. An expression that applies the same functions in turn, such as `10↑⍋v`, takes the general path, while the train `10↑⍋ v` takes the idiom. The glyph pages list each idiom under "Fast combinations", and `•explain` reports each one taken, by the name `Idiom::name` gives.
use super::*;

/// A combination of functions with a faster algorithm.
#[derive(Debug)]
pub(super) enum Idiom {
    /// Some items of a grade or a sort, such as `(10↑)∘⍋` or `↑∘<`. They're selected without ordering the rest. `grade` is true for `⍋` and `⍒`, and `down` for `⍒` and `>`.
    Selected { grade: bool, down: bool, pick: Pick },
    /// `⍋∘⍋` or `⍋∘⍒`: each item's rank. The inner grade is a permutation, whose grade is its inverse. The idiom inverts it directly instead of grading it again.
    Ranks { down: bool },
    /// `(≢⊢)⌸`: the size of each group, counted without building the groups.
    Counts,
    /// `(f/⊢)⌸` or `(f⌿⊢)⌸` for `f` of `+ ⌈ ⌊`, or `×` of floats: each group of a vector folded, with no call of the operand for each group.
    Folds(Primitive),
    /// Monadic `-.×`: the determinant, from one factorization instead of the expansion by minors.
    Determinant,
    /// Monadic `+.×`: the permanent, by rustymath's `permanent_with` instead of the expansion by minors.
    Permanent,
}

/// What `Selected` takes from the order: `k` items from the front, or from the back when `k` is negative, or the item at one position.
#[derive(Clone, Copy, Debug)]
pub(super) enum Pick { Take(i64), Item(i64) }

/// How `g` orders its argument, where it's a primitive that orders it (`Primitive::orders`). An atop of a selection with such a `g` can be an idiom (`Idiom::Selected`, `Idiom::Ranks`). `operators::train_call` leaves any train whose last tine orders its argument to be built, with its idiom.
pub(super) fn orders(g: &Function) -> Option<(bool, bool)> { g.as_primitive()?.orders() }

/// The idiom that `node` is, if any.
pub(super) fn recognise(node: &FunctionNode) -> Option<Idiom> {
    match node {
        FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(f), Operand::Function(g)]) => {
            let (grade, down) = orders(g)?;
            let pick = match f.node() {
                FunctionNode::Primitive(Primitive::Grade(false)) if grade => return Some(Idiom::Ranks { down }),
                FunctionNode::Primitive(Primitive::Take) => Pick::Item(0),
                FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(s)]) => {
                    let k = a.as_number()?.integer().ok()?;
                    match s.as_primitive()? { Primitive::Take => Pick::Take(k), Primitive::Index | Primitive::Mix => Pick::Item(k), _ => return None }
                }
                _ => return None,
            };
            Some(Idiom::Selected { grade, down, pick })
        }
        FunctionNode::Composed(OperatorKind::Product, _) => match node.operation()? {
            Operation::MINUS_TIMES => Some(Idiom::Determinant),
            Operation::PLUS_TIMES => Some(Idiom::Permanent),
            _ => None,
        },
        FunctionNode::Modified(OperatorKind::Key, Operand::Function(f)) => {
            let FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(r), Operand::Function(i)]) = f.node() else { return None };
            if !matches!(i.as_primitive()?, Primitive::Identity(false)) { return None; }
            match r.node() {
                FunctionNode::Primitive(Primitive::Tally) => Some(Idiom::Counts),
                FunctionNode::Fold(g, kind) if !kind.scan => match g.as_primitive()? {
                    p if crate::pervasive::folds(p).is_some() => Some(Idiom::Folds(p)),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

impl Idiom {
    /// The combination's name, as `•explain` reports it.
    fn name(&self) -> &'static str {
        match self {
            Idiom::Selected { grade: true, .. } => "partial grade", Idiom::Selected { grade: false, .. } => "partial sort", Idiom::Ranks { .. } => "ranks",
            Idiom::Counts => "group counts", Idiom::Folds(_) => "group folds", Idiom::Determinant => "determinant", Idiom::Permanent => "permanent",
        }
    }
    /// Records the combination for `•explain`, where it's on. `f` is the function that the combination computes.
    pub(super) fn taken(&self, f: &Function) { if crate::explain::recording() { crate::explain::fast(self.name(), f.bpl()); } }
    /// Records that the combination declined arguments of `items` items that don't suit it, and that the general path runs instead (`rustymath::events`).
    pub(super) fn declined(&self, items: usize) { rustymath::events::declined(self.name(), "arguments that don't suit it", items) }
    /// The idiom's result for these arguments, or `None` where they don't suit it. `f` is the function that the idiom computes. Each of these idioms takes one argument. A call with two takes the general path, with no decline recorded.
    #[inline(never)]
    pub(super) fn call(&self, f: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<Bound, Error>> {
        if left.is_some() { return None; }
        let result = match *self {
            Idiom::Selected { grade, down, pick } => selected(grade, down, pick, right, span),
            Idiom::Ranks { down } => ranks(down, right, span),
            // Key runs these in its own path (`key`), with its validation and classes.
            Idiom::Counts | Idiom::Folds(_) => return None,
            Idiom::Determinant => determinant(right),
            Idiom::Permanent => permanent(right),
        };
        if result.is_some() { self.taken(f); } else { self.declined(right.len()); }
        result
    }
}

/// The items of `right`'s order that `pick` names, through rustymath's partial selection.
fn selected(grade: bool, down: bool, pick: Pick, right: &Value, span: &Span) -> Option<Result<Bound, Error>> {
    if right.shape().len() != 1 || right.keys(0).is_some() || crate::transform::carried(None, right) { return None; }
    let n = right.len();
    let (range, item) = match pick {
        Pick::Take(k) if k.unsigned_abs() <= n as u64 => {
            let m = k.unsigned_abs() as usize;
            (if k < 0 { n - m..n } else { 0..m }, false)
        }
        Pick::Item(i) => {
            let i = rustymath::layout::position(i, n)?;
            (i..i + 1, true)
        }
        Pick::Take(_) => return None,
    };
    // Large runs cost more to comparison-sort than grading the whole vector by radix/counting.
    if range.len() > n / 4 { return None; }
    let crate::array::Items::Compact(view) = right.as_items() else { return None };
    let order = rustymath::search::view_selected(view, range, down)?;
    Some((|| {
        let positions = Value::positions_u32([order.len()], n, order).and_then(|p| p.along_first_of(right)).described("invalid selected order")?;
        let selected = if grade { positions } else {
            let selector = Value::new([1], vec![positions]).described("invalid selected order")?;
            Primitive::Index.call(Some(&selector), right, span)?
        };
        Ok(Bound::from(if item { crate::primitive::pick(&Value::integer(0), &selected, session().prototype.get(), span)? } else { selected }))
    })())
}

/// Each major cell's rank in the order that `⍋` or, where `down`, `⍒` gives: the inverse of the grade's permutation.
fn ranks(down: bool, right: &Value, span: &Span) -> Option<Result<Bound, Error>> {
    use {crate::array::Items, rustymath::buffer::View};
    if right.shape().is_empty() || right.keys(0).is_some() || crate::transform::carried(None, right) { return None; }
    let grade = match Primitive::Grade(down).call(None, right, span) { Ok(g) => g, Err(e) => return Some(Err(e)) };
    let Items::Compact(View::Integers(ints)) = grade.as_items() else { return Some(Primitive::Grade(false).call(None, &grade, span).map(Bound::from)) };
    let n = grade.len();
    let ranks = rustymath::with_ints!(ints, |positions| rustymath::search::inverse_permutation(positions.iter().map(|&p| i64::from(p) as usize)));
    Some(Value::positions_u32([n], n, ranks).and_then(|r| r.along_first_of(right)).described("invalid ranks").map(Bound::from))
}

/// Monadic `-.×`. A square matrix takes `primitive::determinant`. A running level takes a call on its carried values before the idiom (`transform::call`).
fn determinant(right: &Value) -> Option<Result<Bound, Error>> {
    let [m, n] = matrix(right)?;
    if m != n || n == 0 { return None; }
    crate::primitive::determinant(right, n).transpose().map(|d| d.map(Bound::from))
}

/// Monadic `+.×` of a square matrix of fewer than 64 rows of numbers, by rustymath's `permanent_with` at the matrix's own field, which `primitive::determinant` also reads. An integer permanent wraps where the call wraps (`Number::wrapped`).
fn permanent(right: &Value) -> Option<Result<Bound, Error>> {
    use crate::number::{read, with_field, Domain, Field};
    if crate::transform::carried(None, right) { return None; }
    let [m, n] = matrix(right)?;
    if m != n || n >= 64 { return None; }
    let (domain, width) = Domain::of_arrays(&[right])?;
    Some((|| {
        let settings = crate::number::Numeric::current().for_values([right]).map_err(Error::domain)?;
        let p = with_field!(domain, [Exact, Single, Double, Complex], T => rustymath::linalg::exact::permanent(&read::<T>(right)?, n)?.number(width).wrapped(settings));
        Ok(Bound::from(Value::Number(p)))
    })())
}
