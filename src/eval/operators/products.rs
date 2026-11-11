//! Outer products (`⊗`), and `f.g` with one or two arguments: inner products, determinants, permanents and expansions by minors.
use super::*;

pub(in crate::eval) fn outer(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| Error::new(ErrorKind::Syntax, "outer product needs a left argument"))?;
    if left.is_atom() && right.is_atom() { return operand.call(Some(left), right, span); }
    if let Some(p) = pervasive_primitive(operand, true) { return Ok(Bound::from(p.outer(left, right, span)?)); }
    outer_items(operand, left, right, span)
}

/// The outer product's general path: `operand` on each pair of an item of `left` and an item of `right` (`each_pair`).
// Out of line, as `each_items` is, so that `outer`'s frame holds only its fast paths.
#[inline(never)]
fn outer_items(operand: &Function, left: &Value, right: &Value, span: &Span) -> Result<Bound, Error> {
    let agreement = Agreement::outer(left.layout(), right.layout()).described("outer product is too large")?;
    // A batched try reads each argument laid over the result's frame (`batch::Arguments::laid`), as Each's does (`items_whole`).
    let whole = || crate::batch::Arguments::laid(&agreement, Some(items_whole(left)), items_whole(right));
    each_pair(operand, agreement.len, agreement.layout.clone(), |i| Ok(agreement.values(Some(left), right, i)), whole, None, span)
}

/// `f.g` with function operands. Primitives call the operation that they name (`Operation::call`). Other operands route to a running level, then take one batched call of `g` (`by_triples`), or else the general path (`Contraction::items`).
pub(super) fn inner(f: &Function, g: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if let (Some(pf), Some(pg)) = (f.dyadic_primitive(), g.dyadic_primitive()) {
        return Operation::Product(pf, pg).call(Pairing::Axis(None), left, right, span).map(Bound::from);
    }
    if let Some(result) = crate::transform::call(crate::transform::Op::Product, left, right, span) { return result.map(Bound::from); }
    let Some(left) = left else { return expansion(f, g, right, span) };
    let c = Contraction::new(left, right)?;
    if c.ordered && !left.is_singleton() && !right.is_singleton() {
        if let Some(result) = by_triples(f, g, left, right, &c, span) { return result.map(Bound::from); }
    }
    c.items(left, right, f, g, span).map(Bound::from)
}

/// The sizes and layout of `left f.g right`, which every route of a product reads first: `n` pairs along the contraction for each of `rows` rows of `left` and `cols` columns of `right`, giving `size` items laid out by `layout`. `positions` reads the items of `right` along the contraction in the order that `left` pairs them. `ordered` where the lengths agree, `positions` reads them in order, and there are items to compute, as routes on whole arrays need.
pub(crate) struct Contraction { nx: usize, n: usize, rows: usize, cols: usize, size: usize, layout: crate::array::Layout, positions: Mapping, ordered: bool }

impl Contraction {
    /// The contraction of the last axis of `left` with the first axis of `right`. A singleton argument pairs with every item along the other's axis. Other lengths that differ give LENGTH ERROR.
    pub(crate) fn new(left: &Value, right: &Value) -> Result<Self, Error> {
        let nx = left.shape().last().copied().unwrap_or(1);
        let ny = right.shape().first().copied().unwrap_or(1);
        let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), 0)?;
        if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(Error::new(Kind::Length, "product contraction lengths must agree")); }
        let n = if left.is_singleton() { ny } else { nx };
        let layout = left.layout().contracted(right.layout());
        let size = generated_len(layout.shape())?;
        let rows = generated_len(&left.shape()[..left.shape().len().saturating_sub(1)])?;
        let cols = generated_len(&right.shape()[usize::from(!right.is_unit())..])?;
        generated_len(&[n.max(1), cols.max(1)])?;
        Ok(Self { nx, n, rows, cols, size, ordered: n > 0 && size > 0 && positions.in_order(ny, &[nx]), layout, positions })
    }
    /// The general path: each item of the result is the left operand's fold of the right operand's results on the pairs of a row of `left` and a column of `right`. An empty result, or an empty contraction, calls the operands only for prototypes, in prototype mode.
    pub(crate) fn items(&self, left: &Value, right: &Value, f: &Function, g: &Function, span: &Span) -> Result<Value, Error> {
        let Self { nx, n, rows, cols, size, .. } = *self;
        let item = |a: &Value, offset| if n == 0 || a.is_empty() { a.prototype() } else { a.at(if a.is_singleton() { 0 } else { offset }) };
        let mut results = Vec::with_capacity(size.max(1));
        for i in 0..rows.max(1) {
            let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
            for k in 0..n.max(1) {
                for (j, column) in columns.iter_mut().enumerate() {
                    let rk = if n == 0 { 0 } else { self.positions.index(k) };
                    column.push(session().prototype_mode(size == 0 || n == 0, || {
                        g.call_with(Some(item(left, i * nx + k)), item(right, rk * cols + j), span)?.array(span)
                    })?);
                }
            }
            for column in columns {
                let paired =
                    if n == 0 { Value::empty(vec![0], column[0].prototype()) } else { Value::new(vec![n], column) }.described("invalid product cell")?;
                results.push(session().prototype_mode(size == 0, || fold(f, FoldKind::REDUCE, None, None, &paired, span))?);
            }
        }
        if self.layout.shape().is_empty() { return Ok(results.remove(0)); }
        let prototype = results[0].zeroed();
        if size == 0 { results.clear(); }
        self.layout.collect(results, prototype).described("invalid inner product result")
    }
}

/// `left f.g right` as one batched call of `g` on every triple of a row of `left`, a position along the contraction and a column of `right`, followed by the fold of `f` along the contraction by the fold's own route, for each chunk of rows (`by_rows`). `None` where a chunk's run falls back. After a fallback, the loop over rows runs from the start and keeps effects in order.
fn by_triples(f: &Function, g: &Function, left: &Value, right: &Value, c: &Contraction, span: &Span) -> Option<Result<Value, Error>> {
    if !crate::batch::tries(c.rows * c.n * c.cols) { return None; }
    by_rows(c, left, right, |x, y| {
        let agreement = Agreement::new(x.layout(), y.layout()).ok()?;
        let frame = agreement.layout.clone();
        let products = crate::batch::tried(g, || crate::batch::Arguments::laid(&agreement, Some((x, 0)), (y, 0)), &frame, false, span)?;
        Some(products.and_then(|p| fold(f, FoldKind::REDUCE, Some(&Value::integer(1)), None, &p, span)))
    })
}

/// `left f.g right` through `product`, which takes rows of `left` laid out as rows by contraction by 1 and `right` laid out as 1 by contraction by columns, and gives their products folded along the contraction. The rows go in chunks whose products hold at most `Sizes::batch` items, or one row where a row's products hold more, and the chunks' results join along the rows. `None` where `product` gives `None` for a chunk.
fn by_rows(
    c: &Contraction, left: &Value, right: &Value, mut product: impl FnMut(Value, Value) -> Option<Result<Value, Error>>) -> Option<Result<Value, Error>> {
    use {crate::array::Layout, rustymath::gather::Steps};
    let Contraction { rows, n, cols, .. } = *c;
    let per = (crate::settings::sizes().batch / n.saturating_mul(cols).max(1)).max(1);
    let y = right.relaid(Layout::from(vec![1, n, cols])).ok()?;
    // One chunk lays out `left` as it is. Chunks take their rows from `left`'s items in order.
    let flat = if per < rows { Some(left.relaid(Layout::from(vec![rows * n])).ok()?) } else { None };
    let mut parts = Vec::with_capacity(rows.div_ceil(per));
    for start in (0..rows).step_by(per) {
        let count = per.min(rows - start);
        let x = match &flat {
            None => left.relaid(Layout::from(vec![rows, n, 1])), Some(flat) => crate::fusion::copied(
                flat, Layout::from(vec![count * n]), &rustymath::agreement::Mapping::from(vec![Steps::Stride { start: start * n, len: count * n, step: 1 }]),
                None).and_then(|x| x.relaid(Layout::from(vec![count, n, 1])))
        };
        let x = match x.described("invalid product rows") { Ok(x) => x, Err(e) => return Some(Err(e)) };
        parts.push(match product(x, y.clone())? { Ok(p) => p, Err(e) => return Some(Err(e)) });
    }
    let whole = if parts.len() == 1 { parts.pop().expect("one chunk") } else {
        let mut joined = Gather::new(&parts.iter().collect::<Vec<_>>(), rows * cols);
        for part in &parts { joined.append(part); }
        match joined.finish(Layout::from(vec![rows, cols]), || parts[0].prototype()).described("invalid product rows") {
            Ok(v) => v, Err(e) => return Some(Err(e))
        }
    };
    Some(if c.layout.shape().is_empty() { Ok(whole.at(0)) } else { whole.relaid(c.layout.clone()).described("invalid inner product result") })
}

/// `left f.g right` for primitives `f` and `g`, as `Operation::run` gives it: rustymath's inner-product kernel where it applies, then `g` once on rows of `left` and on `right` laid out against each other, followed by `f`'s fold along the contraction, where `g` is pervasive (`by_rows`), and otherwise the general path (`Contraction::items`). A running level takes each call that these make.
pub(crate) fn primitive_product(f: Primitive, g: Primitive, left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    let c = Contraction::new(left, right)?;
    if c.ordered && !left.is_unit() && !right.is_unit() && !crate::transform::carried(Some(left), right) {
        let plan = crate::plan::product_on(f, g, left, right);
        if let Ok(product) = rustymath::linalg::ProductPlan::dot([&**left.layout(), &**right.layout()])
            && let Some(result) = crate::pervasive::inner(&plan, left, right, &product, c.layout.clone()) {
                let result = result?;
                return Ok(if c.layout.shape().is_empty() { result.at(0) } else { result });
            }
    }
    if c.ordered && !left.is_singleton() && !right.is_singleton() && g.facts(true).pervasive() {
        let fold = Operation::Fold(f, FoldKind::REDUCE);
        if let Some(result) = by_rows(&c, left, right, |x, y| Some(g.call(Some(&x), &y, span).and_then(|p| fold.call(Pairing::Axis(Some(1)), None, &p, span))))
        { return result; }
    }
    c.items(left, right, &Function::primitive(f), &Function::primitive(g), span)
}

/// Monadic `f.g` for primitives `f` and `g`, as `Operation::run` and the combinations `Determinant` and `Permanent` give it: for a square matrix of numbers, the determinant of `-.×` (`primitive::determinant`) and, below 64 rows, the permanent of `+.×`, and otherwise the expansion by minors that function operands take (`expansion`), whose calls of the operation by its primitives a running level takes.
pub(crate) fn primitive_expansion(f: Primitive, g: Primitive, right: &Value, span: &Span) -> Result<Value, Error> {
    let route = match Operation::Product(f, g) {
        Operation::MINUS_TIMES => Some(crate::events::routes::Determinant), Operation::PLUS_TIMES => Some(crate::events::routes::Permanent), _ => None
    };
    if let Some(route) = route {
        let square = matrix(right).filter(|&[m, n]| m == n && n > 0 && !crate::transform::carried(None, right)).map(|[_, n]| n);
        let fast = match (route, square) {
            (crate::events::routes::Determinant, Some(n)) => crate::primitive::determinant(right, n)?,
            (crate::events::routes::Permanent, Some(n)) if n < 64 => permanent(right, n)?, _ => None
        };
        let Some(value) = fast else {
            crate::events::declined(route, "arguments that don't suit it", right.len());
            return expansion(&Function::primitive(f), &Function::primitive(g), right, span)?.array(span);
        };
        if crate::events::running() { crate::explain::fast(route, format!("{}.{}", f.glyph(), g.glyph())); }
        return Ok(value);
    }
    expansion(&Function::primitive(f), &Function::primitive(g), right, span)?.array(span)
}

/// The permanent of the `n`×`n` matrix `right`, by rustymath's `permanent_with` at the matrix's own field, which `primitive::determinant` also reads, or `None` where it doesn't hold numbers. Machine integers give the machine integer that the result wraps to (`Number::machine_result`).
fn permanent(right: &Value, n: usize) -> Result<Option<Value>, Error> {
    use crate::number::{Domain, Field, read, with_field};
    let Some((domain, width)) = Domain::of_arrays(&[right]) else { return Ok(None) };
    let settings = crate::number::Numeric::for_values([right]);
    let p = with_field!(domain, [Exact, Single, Double, Complex], T => rustymath::linalg::exact::permanent(&read::<T>(right)?, n)?.number(width).machine_result(&[right], settings));
    Ok(Some(Value::Number(p)))
}

/// `op` on each matrix of the stack `right`, or on each pair of matrices of the stacks `left` and `right`, as `op⍤2` pairs them, as `Operation::run` gives it: rustymath's kernels on whole stacks where they apply (`primitive::stacked`), and otherwise `op` on each matrix or pair, whose results mix as Rank mixes them. A running level takes each of those calls.
pub(crate) fn matrices(op: Operation, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    if !crate::transform::carried(left, right) { if let Some(result) = crate::primitive::stacked(op, left, right) { return result; } }
    let pairs = CellPairs::new(left, right, [left.map_or(0, |a| within(a.shape().len(), 2)), within(right.shape().len(), 2)])?;
    assembled(pairs.agreement.len, pairs.agreement.layout.clone(), Some("invalid rank result"), |i, _| {
        let (x, y) = pairs.pair(i)?;
        op.call(Pairing::Axis(None), x.as_ref(), &y, span)
    })
}

/// Monadic `f.g` with function operands (`expanded`).
fn expansion(f: &Function, g: &Function, right: &Value, span: &Span) -> Result<Bound, Error> {
    expanded(right, &mut |x, y| {
        check()?;
        match y { None => fold(g, FoldKind::REDUCE, None, None, x, span), Some(y) => inner(f, g, Some(x), y, span)?.array(span) }
    }).map(Bound::from)
}

/// Monadic `f.g` of `right`, as J defines it: the expansion of a matrix by minors along its first column. A vector is one column, and an atom one item. `call(x, Some(y))` is `x f.g y`, and `call(x, None)` is `g/x`. `expansion` calls it. `-.×` is the determinant and `+.×` the permanent, which `primitive_expansion` computes by faster algorithms.
pub(crate) fn expanded(right: &Value, call: &mut dyn FnMut(&Value, Option<&Value>) -> Result<Value, Error>) -> Result<Value, Error> {
    crate::program::trace::per_item(None, right)?;
    let [m, n] = matrix(right).ok_or_else(|| Error::new(Kind::Rank, "monadic f.g needs a matrix, a vector or a number"))?;
    if m > 64 { return Err(Error::new(Kind::Limit, "monadic f.g expands at most 64 rows")); }
    let items: Vec<Value> = (0..m * n).map(|i| right.at(i)).collect();
    let rows = if m == 0 { 0 } else { u64::MAX >> (64 - m) };
    expand(&items, [m, n], rows, &right.prototype(), &mut HashMap::new(), call)
}

/// The rows and columns that monadic `f.g` reads `right` as: a vector is one column, and a number one item.
pub(in crate::eval) fn matrix(right: &Value) -> Option<[usize; 2]> {
    match *right.shape() { [] => Some([1, 1]), [m] => Some([m, 1]), [m, n] => Some([m, n]), _ => None }
}

/// The expansion of the rows in the set `rows` of the `m`×`n` matrix `items`, from column `m` minus the size of `rows` on. While columns are left, it is `call` between that column's items in `rows` and the expansions without each of those rows, as `f.g`. With no columns left, it is `call` on no items, as `g/`. `memo` holds the expansion of each set of rows already expanded.
fn expand(
    items: &[Value], [m, n]: [usize; 2], rows: u64, prototype: &Value, memo: &mut HashMap<u64, Value>,
    call: &mut dyn FnMut(&Value, Option<&Value>) -> Result<Value, Error>) -> Result<Value, Error> {
    if let Some(value) = memo.get(&rows) { return Ok(value.clone()); }
    let vector = |items: Vec<Value>| {
        if items.is_empty() { Value::empty(vec![0], prototype.clone()) } else { Value::new(vec![items.len()], items) }.described("invalid expansion")
    };
    let column = m - rows.count_ones() as usize;
    let value = if column == n { call(&vector(vec![])?, None)? } else {
        let set: Vec<usize> = (0..m).filter(|&i| rows >> i & 1 == 1).collect();
        let first = vector(set.iter().map(|&i| items[i * n + column].clone()).collect())?;
        let minors = set.iter().map(|&i| expand(items, [m, n], rows & !(1 << i), prototype, memo, call)).collect::<Result<Vec<_>, _>>()?;
        call(&first, Some(&vector(minors)?))?
    };
    memo.insert(rows, value.clone());
    Ok(value)
}
