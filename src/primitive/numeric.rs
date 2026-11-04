//! Numeric primitives outside the pervasive kernels: matrix division and decompositions, encode and decode, complex parts, number formatting, the Lambert W function.

use super::*;
use crate::execution::check;
use crate::number::{Domain, Field, read, with_field};
use num_complex::Complex64;
use num_rational::BigRational;
use rustymath::agreement::{Frames, Missing};
use rustymath::array::ArrayView;
use rustymath::buffer::{Buffer, FloatWidth, IntBuf};
use rustymath::linalg::{self, Factor, Inner, PlusTimes};
use rustymath::{self as rm, buffer::View, error::Kind, facts::{Formula, Range, Variant}, operation::Kernel, tolerant as tol};

pub(super) fn complex_parts(origin: &Origin<&[Value]>, call: &CallView<'_>, polar: bool) -> Result<Outputs<Value>, Error> {
    let root = call.layout().arguments().last().expect("a parts argument");
    let right = origin.root_input(root)?.into_owned();
    let convert = polar && right.kind().is_some_and(|kind| kind.is_exact() && (!call.mode.dynamic || kind.fixed_width().is_some()));
    let right =
        if convert { Cow::Owned(origin.apply(Operation::Convert(Mark::Real), std::slice::from_ref(&right))?) } else { Cow::Borrowed(&right) };
    let part = |primitive| {
        origin.apply(Operation::Primitive { primitive, pairing: Pairing::Axes }, std::slice::from_ref(right.as_ref()))
    };
    let (left, right) = if polar { (part(Primitive::Math(Math::Magnitude))?, part(Primitive::Special(Special::Angle))?) }
    else { (part(Primitive::Special(Special::Real))?, part(Primitive::Special(Special::Imag))?) };
    let column = |value: Value| {
        let axes = (0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }).chain([ViewAxis { input: 0, axis: None }]).collect();
        let destinations = (0..value.rank()).collect();
        origin.apply(Operation::Read(ReadOp::Axes { axes, destinations }), &[value])
    };
    let value = origin.apply(Operation::Primitive { primitive: Primitive::Ravel, pairing: Pairing::Axes }, &[column(left)?, column(right)?])?;
    Ok(Outputs::one(value))
}

fn format_number(n: &Number, precision: isize) -> Result<String, Error> {
    use num_traits::Signed;
    if n.as_complex().is_some() { return Err(Error::domain("specified format requires real numbers")); }
    if n.is_infinite() { return Ok(n.to_string()); }
    let digits = precision.unsigned_abs();
    let mut text = if precision >= 0 && n.is_exact() {
        let scaled = (n.as_exact().unwrap() * num_bigint::BigInt::from(10).pow(digits as u32)).round().to_integer();
        let mut s = scaled.abs().to_string();
        if digits != 0 {
            if s.len() <= digits { s.insert_str(0, &"0".repeat(digits + 1 - s.len())); }
            s.insert(s.len() - digits, '.');
        }
        if scaled.is_negative() { s.insert(0, '¯'); }
        s
    } else {
        let y = n.to_complex()?.re;
        let exponent = if y == 0.0 { 0 } else { y.abs().log10().floor() as i32 };
        let places = if precision >= 0 { digits as i32 } else { digits as i32 - 1 - exponent };
        let scale = 10_f64.powi(places);
        let scaled = y * scale;
        let y = if scale.is_finite() && scale != 0.0 && scaled.abs() < 1e16 { scaled.round() / scale } else { y };
        if precision >= 0 { format!("{y:.digits$}") } else { crate::number::scientific(&format!("{:.*e}", digits - 1, y)) }
    };
    if !n.is_exact() {
        let mut significant = 0;
        let mut exponent = false;
        text = text.chars().map(|c| {
            if c == crate::number::EXPONENT.1 { exponent = true; }
            if !exponent && c.is_ascii_digit() && (c != '0' || significant != 0) { significant += 1; }
            if !exponent && c.is_ascii_digit() && significant > 16 { '_' } else { c }
        }).collect();
    }
    Ok(text.replace('-', "¯"))
}

/// `⍕right`, or `left⍕right`, where `left` is a precision, a width and precision, or a width and precision for each column. A width of 0 fits a column to its longest number, with one space before each column after the first.
pub(super) fn format_array(left: Option<&Value>, right: &Value) -> Result<Value, Error> {
    use crate::display::Elide;
    let Some(spec) = left else { return right.formatted(Elide::NONE).described("formatted array is too large"); };
    // An empty spec, `⍬` or `''`, gives monadic `⍕`'s text without the markers of numbers' kinds.
    if spec.shape().len() == 1 && spec.is_empty() { return right.formatted(Elide { marks: false, ..Elide::NONE }).described("formatted array is too large"); }
    if spec.shape().len() > 1 { return Err(Error::new(Kind::Rank, "format specification must be a unit or vector")); }
    let spec = spec.elements()
        .map(|e| numeric(&e)?.integer().and_then(|n| isize::try_from(n).map_err(|_| Kind::Limit.into())).described("format specification must be whole"))
        .collect::<Result<Vec<_>, _>>()?;
    let columns = right.shape().last().copied().unwrap_or(1);
    if !matches!(spec.len(), 1 | 2) && spec.len() != 2 * columns {
        return Err(Error::new(Kind::Length, "format needs precision, a width/precision pair, or pairs per column"));
    }
    numeric(&right.prototype())?;
    let specs: Vec<_> = (0..columns).map(|i| match spec.len() { 1 => (0, spec[0]), 2 => (spec[0], spec[1]), _ => (spec[2 * i], spec[2 * i + 1]) }).collect();
    let mut widths = Vec::new();
    for &(width, precision) in &specs {
        if width < 0 { return Err(Error::domain("format width must be nonnegative")); }
        generated_len(&[right.len().max(1), precision.unsigned_abs().max(width as usize).max(1)])?;
        widths.push(if width == 0 { 1 } else { width as usize });
    }
    let text = right.elements().enumerate().map(|(i, e)| {
        let s = format_number(numeric(&e)?, specs[i % columns].1)?;
        if specs[i % columns].0 == 0 { widths[i % columns] = widths[i % columns].max(s.chars().count() + usize::from(i % columns > 0)); }
        Ok(s)
    }).collect::<Result<Vec<_>, Error>>()?;
    let width = widths.iter().sum();
    let mut shape = right.shape().to_vec();
    if let Some(last) = shape.last_mut() { *last = width; }
    else { shape.push(width); }
    generated_len(&shape)?;
    let mut data = Vec::new();
    for (i, s) in text.iter().enumerate() {
        let width = widths[i % columns];
        let len = s.chars().count();
        let s = if len > width { "*".repeat(width) }
        else if specs[i % columns].1 < 0 {
            let leading = usize::from(specs[i % columns].0 == 0 && i % columns > 0);
            format!("{}{s}{}", " ".repeat(leading), " ".repeat(width - len - leading))
        } else { format!("{}{s}", " ".repeat(width - len)) };
        data.extend(s.chars().map(Value::Character));
    }
    Value::from_parts(shape, data, Value::Character(' ')).described("invalid formatted result")
}

/// `⌹` on a matrix, a vector or a number: the solution of `right x = left`, or `right`'s inverse without `left`. A vector is one column, and a number a 1×1 matrix. Floats and complex numbers run through rustymath's [`Stacked`](linalg::Stacked), as stacks of matrices do, where a square matrix with no negligible LU pivot solves or inverts through LU and any other gives the least-squares solution or the pseudoinverse. Exact numbers stay exact, through row reduction.
pub(super) fn matrix_divide(left: Option<&Value>, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let ([m, n, k], layout, scalar) = crate::plan::divided_layout(right.layout(), left.map(Value::layout))?;
    generated_len(&[n, k])?;
    let (domain, width) = match left { Some(x) => domain_of(&[right, x], numeric), None => domain_of(&[right], numeric) }?;
    if n == 0 {
        let prototype = if domain == Domain::Exact { right.prototype() } else { Value::Number(Number::float(0.0, width)) };
        return layout.collect(vec![], prototype).described("invalid matrix shape");
    }
    if domain != Domain::Exact {
        let matrix = match right.shape() { [] => Layout::from([1, 1]), [_] => right.layout().concat(&Layout::from([1])), _ => right.layout().clone() };
        let args: Vec<_> =
            [(right, matrix)].into_iter().chain(left.map(|x| (x, if x.shape().is_empty() { Layout::from([1]) } else { x.layout().clone() }))).collect();
        let x = inexact(&args, (domain, width), |s, v| match v {
            [a, b] => s.solve(a.clone(), b.clone()), [a] => s.pinv(a.clone()), _ => unreachable!("a view for each argument")
        }).map_err(|s| s.raised())?;
        let x = value(x.into_compact().1, layout)?;
        return Ok(if scalar { x.at(0) } else { x });
    }
    // Monadic `⌹` gives an inverse from one factorization where it can, and a pseudoinverse otherwise.
    if left.is_none() { if let Some((inverse, _)) = matrix_inverse(right, false, numeric)? { return Ok(inverse); } }
    let positions = left.map(|x| Mapping::contract(right.layout(), 0, x.layout(), 0)).transpose()?;
    let a = read::<BigRational>(right)?;
    let x = match ordered(left.map(|x| read::<BigRational>(x)).transpose()?, positions.as_ref(), m, k) {
        Some(b) => linalg::exact::solve(&a, &b, [m, n], k), None => linalg::exact::pinv(&a, [m, n])
    }?;
    matrix_result(x, layout, scalar, width)
}

/// `⌹a` and the determinant of `a`, from one factorization, for a square matrix or a number `a`, or for each matrix in the last two axes of `a` where `stacked`, as `∂`'s rules for `⌹` and `-.×` need them. A stack's determinants keep its frame's axes. `None` where `⌹` gives the pseudoinverse of some matrix: a singular exact matrix, or a float matrix with a negligible LU pivot. Floats and complex numbers go through rustymath's [`Stacked`](linalg::Stacked), as `⌹` reads them. Exact matrices factor on rustymath's worker threads, which check the running evaluation's stops within each.
pub(crate) fn matrix_inverse(a: &Value, stacked: bool, numeric: Numeric) -> Result<Option<(Value, Value)>, Error> {
    let r = a.shape().len();
    let (frame, n) = match *a.shape() { [] | [1] => (0, 1), [.., m, n] if m == n && (stacked || r == 2) => (r - 2, n), _ => return Ok(None) };
    let (domain, width) = domain_of(&[a], numeric)?;
    let (inverse, determinant) = (a.layout().axes((0..frame).chain((frame..r).rev())), a.layout().axes(0..frame));
    if domain == Domain::Exact {
        let frames = Frames::new(&[(&**a.layout(), r - frame)])?;
        let items = read::<BigRational>(a)?;
        let Some(cells) = linalg::cells(&frames, |i, _: &mut ()| linalg::exact::inverse(&items[frames.items(0, i, n * n)], n))?.into_iter()
            .collect::<Result<Option<Vec<_>>, _>>()?
        else { return Ok(None); };
        let (x, d): (Vec<_>, Vec<_>) = cells.into_iter().unzip();
        return Ok(Some((matrix_result(x.concat(), inverse, a.is_atom(), width)?, matrix_result(d, determinant, frame == 0, width)?)));
    }
    let matrices = if r < 2 { Layout::from([1, 1]) } else { a.layout().clone() };
    let Some((x, d)) = inexact(&[(a, matrices)], (domain, width), |s, v| s.inverse(v[0].clone())).map_err(|s| s.raised())? else { return Ok(None) };
    let (x, d) = (value(x.into_compact().1, inverse)?, value(d.into_compact().1, determinant)?);
    Ok(Some((if a.is_atom() { x.at(0) } else { x }, if frame == 0 { d.at(0) } else { d })))
}

/// A result of `⌹`, `x`, laid out by `layout`, or its one item where it's `scalar`.
fn matrix_result<T: Field>(mut x: Vec<T>, layout: Layout, scalar: bool, width: FloatWidth) -> Result<Value, Error> {
    if scalar { return Ok(Value::Number(x.swap_remove(0).number(width))); }
    T::array(x, layout.shape().to_vec(), width)?.with_layout(layout).described("invalid matrix result")
}

/// `f` on `args`, the arguments of a matrix function with no exact form, each with the layout it takes, through rustymath's [`Stacked`](linalg::Stacked), where exact numbers compute as floats of `width`. Compact storage is read in place. Mixed storage gives its numbers in the field of `domain`.
fn inexact<R>(
    args: &[(&Value, Layout)], (domain, width): (Domain, FloatWidth), f: impl FnOnce(&linalg::Stacked, &[ArrayView<'_>]) -> Result<R, rustymath::error::Error>
) -> Result<R, Stopped> {
    use rustymath::buffer::{Buffer, Element};
    let gathered = |a: &Value| match a.view() {
        Some(_) => Ok(None), None => with_field!(domain, [Single, Double, Complex], T => read::<T>(a).map(|x| Some(T::buffer(x.into_owned()))))
    };
    let owned = args.iter().map(|(a, _)| gathered(a)).collect::<Result<Vec<_>, _>>().map_err(Stopped::Checked)?;
    let views: Vec<_> = args.iter().zip(&owned)
        .map(|((a, layout), o)| ArrayView::compact(layout, o.as_ref().map_or_else(|| a.view().expect("compact storage"), Buffer::view))).collect();
    f(&linalg::Stacked::new(width), &views).map_err(Stopped::Array)
}

/// `f⍤2` on stacks of matrices, through the rustymath function that `f` uses on one matrix. `op` names `f`: dyadic `⌹`, or `f.g` with a product kernel for the pair, or monadic `⌹` or `-.×`. `x` is the left stack, absent for the monads. `⌹` and `-.×` run through rustymath's [`Stacked`](linalg::Stacked), which pairs the stacks' frames as Rank pairs them and splits the cells across rustymath's threads, and `f.g` through [`stacked_product`]. `None` where `f⍤2` takes its general path: for exact matrices, which it computes exactly, for empty stacks, and for shapes, frames or keys that don't fit, whose errors it raises.

/// What stops a matrix function on stacks: a rustymath error, after which `f⍤2` takes its general path and one matrix raises it, or an item that isn't a number. An interrupt or a passed deadline comes as a rustymath error, and raises at once.
enum Stopped { Array(rustymath::error::Error), Checked(Error) }

impl From<rustymath::error::Error> for Stopped { fn from(e: rustymath::error::Error) -> Self { Self::Array(e) } }

impl Stopped {
    /// The error that a matrix function on one matrix raises.
    fn raised(self) -> Error { match self { Self::Array(e) => Error::new(e, ""), Self::Checked(e) => e } }
}

/// `buffer`, rustymath's result, as a value laid out by `layout`. Complex items with no imaginary parts become floats, as [`Value::complex`] makes them.
fn value(buffer: rustymath::buffer::Buffer, layout: Layout) -> Result<Value, Error> {
    let shape = layout.shape().to_vec();
    match buffer {
        rustymath::buffer::Buffer::Complexes(c) => Value::complex(shape, c), b => Value::imported_buffer(shape, b)
    }?.with_layout(layout).described("invalid matrix result")
}

/// `x f.g⍤2 y` on stacks of compact matrices with plain layouts, through rustymath's product kernel for the pair (`pervasive::inner`), which pairs the stacks' frames and splits the cells across rustymath's threads. `None` where `f.g⍤2` takes its general path, as for [`stacked`], and where the pair has no kernel for the stacks' items.

/// `kind •decompose m` factors the matrix `m`, and gives the factors as a record. `kind` is `"svd"`, `"qr"`, `"eigen"`, `"cholesky"` or `"lu"`, as text or in a record of options that `Decomposition` describes. A real matrix gives real factors, apart from complex eigenvalues and their eigenvectors. The matrix factors in the [`Domain`] of its items, or in floats of the default width where they are all exact.
pub(crate) fn decompose(crate::system::HostCall { left, right, site: span, numeric }: crate::system::HostCall<'_>) -> Result<Value, Error> {
    let d = Decomposition::new(left)?;
    let &[m, n] = right.shape() else { return Err(Error::new(Kind::Rank, "•decompose needs a matrix")) };
    if m == 0 || n == 0 { return Err(Error::domain("•decompose needs a nonempty matrix")); }
    if d.square() && m != n { return Err(Error::new(Kind::Length, "eigen, cholesky and lu need a square matrix")); }
    let (names, cells) = factors(&d, right, None, span, numeric)?;
    record(names, cells)
}

/// `•decompose`'s left argument: the kind of decomposition, as text or as the `kind` of a record of options. Option `vectors`, true by default, leaves out the vectors of `"svd"` and `"eigen"` where it's false. Option `complete`, false by default, gives `"qr"`'s complete factors where it's true. Option `rank` gives `"svd"`'s first `rank` singular values and vectors, by `truncated_svd`.
struct Decomposition { kind: String, vectors: bool, complete: bool, rank: Option<usize> }

impl Decomposition {
    fn new(left: Option<&Value>) -> Result<Self, Error> {
        let options = crate::data::Options::new("•decompose", left, Some("kind"), &["kind", "vectors", "complete", "rank"])?;
        let kind = options.text("kind", None)?.to_lowercase();
        if !matches!(kind.as_str(), "svd" | "qr" | "eigen" | "cholesky" | "lu") {
            return Err(Error::domain("•decompose takes \"svd\", \"qr\", \"eigen\", \"cholesky\" or \"lu\""));
        }
        let (vectors, complete) = (options.boolean("vectors", true)?, options.boolean("complete", false)?);
        if !vectors && !matches!(kind.as_str(), "svd" | "eigen") { return Err(Error::domain("only svd and eigen can leave out their vectors")); }
        if complete && kind != "qr" { return Err(Error::domain("only qr has complete factors")); }
        let rank = options.number("rank")?;
        if rank.is_some_and(|k| !(k >= 1.0 && k.fract() == 0.0)) { return Err(Error::domain("•decompose rank must be a positive whole number")); }
        if rank.is_some() && kind != "svd" { return Err(Error::domain("only svd takes a rank")); }
        Ok(Self { kind, vectors, complete, rank: rank.map(|k| k as usize) })
    }
    /// Whether the kind needs square matrices: `"eigen"`, `"cholesky"` and `"lu"`.
    fn square(&self) -> bool { matches!(self.kind.as_str(), "eigen" | "cholesky" | "lu") }
}

/// The factors that `d` names, for each matrix in `right`'s last two axes, from rustymath's matrix functions on stacks: their names, and the factors of each matrix in turn, in one list. The matrices factor in the [`Domain`] of their items, or as floats of their width where they're all exact. `"eigen"` gives a Hermitian matrix real eigenvalues, in ascending order, and orthonormal eigenvectors, and any other matrix complex ones: as `hermitian` says for every matrix, or where it's `None`, as each matrix is, so a matrix in a stack gets what it gets alone. A matrix that its factorization fails on, which rustymath gives as NaN, is a DOMAIN error.
fn factors(d: &Decomposition, right: &Value, hermitian: Option<bool>, span: &Span, numeric: Numeric) -> Result<(Vec<&'static str>, Vec<Value>), Error> {
    let (domain, width) = domain_of(&[right], numeric)?;
    let (domain, rank) = (domain.inexact(width), right.shape().len());
    let hermitian = match hermitian {
        Some(h) => h,
        None if d.kind != "eigen" => false,
        None => {
            let n = right.shape()[rank - 1];
            let each = with_field!(domain, [Single, Double, Complex], T => read::<T>(right)?.chunks_exact(n * n).map(|a| linalg::is_hermitian(a, n)).collect::<Vec<_>>());
            match (each.contains(&true), each.contains(&false)) { (true, true) => return mixed(d, right, &each, span, numeric), (h, _) => h }
        }
    };
    let arrays = inexact(&[(right, right.layout().clone())], (domain, width), |s, v| {
        let a = v[0].clone();
        Ok(match d.kind.as_str() {
            "svd" if d.rank.is_none() && !d.vectors => vec![s.svdvals(a)?],
            "svd" => { let (u, values, v) = s.svd(a, d.rank)?; if d.vectors { vec![u, values, v] } else { vec![values] } }
            "qr" => { let (q, r) = s.qr(a, d.complete)?; vec![q, r] }
            "cholesky" => vec![s.cholesky(a)?],
            "lu" => { let (f, p) = s.lu(a)?; vec![s.lu_lower(f.view())?, s.lu_upper(f.view())?, p] }
            _ => match (hermitian, d.vectors) {
                (true, true) => { let (w, v) = s.eigh(a)?; vec![w, v] } (true, false) => vec![s.eigvalsh(a)?],
                (false, true) => { let (w, v) = s.eig(a)?; vec![w, v] } (false, false) => vec![s.eigvals(a)?]
            },
        })
    }).map_err(|s| s.raised())?;
    // rustymath gives NaN for a matrix that a factorization fails on.
    let nan = |i: usize| {
        let v = arrays[i].view();
        match v.dtype() {
            rustymath::buffer::DType::Complex(_) => v.items::<Complex64>().is_ok_and(|x| x.iter().any(|z| z.is_nan())),
            _ => v.items::<f64>().is_ok_and(|x| x.iter().any(|x| x.is_nan())),
        }
    };
    match d.kind.as_str() {
        "svd" if nan(usize::from(d.vectors)) => return Err(Error::domain("svd factorization failed")),
        "cholesky" if nan(0) => return Err(Error::domain("cholesky needs a Hermitian positive definite matrix")),
        "eigen" if nan(0) => return Err(Error::domain("eigen factorization failed")), _ => {}
    }
    let names = match (d.kind.as_str(), d.vectors) {
        ("svd", true) => vec!["u", "s", "v"], ("svd", false) => vec!["s"], ("qr", _) => vec!["q", "r"], ("cholesky", _) => vec!["l"],
        ("lu", _) => vec!["l", "u", "p"], (_, true) => vec!["values", "vectors"], _ => vec!["values"]
    };
    let frame = rank - 2;
    let values = arrays.into_iter().map(|a| {
        let (layout, buffer) = a.into_compact();
        value(buffer, Layout::from(layout.shape().to_vec()))
    }).collect::<Result<Vec<_>, _>>()?;
    let cells = values.iter().map(|v| v.cells(v.shape().len() - frame)).collect::<Result<Vec<_>, _>>().described("invalid factors")?;
    let count = crate::array::count(&right.shape()[..frame]);
    let cells = (0..count).flat_map(|i| cells.iter().map(move |c| c.get(i))).collect::<Result<_, _>>().described("invalid factors")?;
    Ok((names, cells))
}

/// [`factors`] of a stack that mixes Hermitian matrices with others: each part factors as its matrices would alone, and the parts' factors merge back in the stack's order.
fn mixed(d: &Decomposition, right: &Value, hermitian: &[bool], span: &Span, numeric: Numeric) -> Result<(Vec<&'static str>, Vec<Value>), Error> {
    let matrices = right.cells(2).described("invalid matrices")?;
    let part = |h: bool| {
        let chosen = hermitian.iter().enumerate().filter(|&(_, &e)| e == h).map(|(i, _)| matrices.get(i)).collect::<Result<Vec<_>, _>>()
            .described("invalid matrices")?;
        let stack =
            crate::array::assemble_cells(&Layout::from([chosen.len()]), &chosen, &chosen[0], Widening::Items, None, numeric).described("invalid matrices")?;
        factors(d, &stack, Some(h), span, numeric)
    };
    let ((names, yes), (_, no)) = (part(true)?, part(false)?);
    let (k, mut all) = (names.len(), Vec::with_capacity(yes.len() + no.len()));
    let (mut yes, mut no) = (yes.into_iter(), no.into_iter());
    for &h in hermitian { all.extend((if h { &mut yes } else { &mut no }).take(k)); }
    Ok((names, all))
}

/// The record of `factors` under their `names`.
fn record(names: Vec<&'static str>, factors: Vec<Value>) -> Result<Value, Error> {
    crate::keyed::vector(names.into_iter().map(Into::into).collect(), factors).described("invalid factors")
}

/// The shared numeric field and float width for a matrix function.
fn domain_of(arrays: &[&Value], numeric: Numeric) -> Result<(Domain, FloatWidth), Error> {
    Domain::of_arrays(arrays, numeric).ok_or_else(|| Error::domain("expected numeric elements"))
}

/// The `rows`×`columns` matrix that `x` holds row by row, with floats at `width`.
fn matrix<T: Field>(x: Vec<T>, [rows, columns]: [usize; 2], width: FloatWidth) -> Result<Value, ErrorKind> { T::array(x, vec![rows, columns], width) }

/// `b`, an `m`×`k` matrix held row by row, with row `i` taken from row `positions[i]` when there are positions.
fn ordered<'a, T: Clone>(b: Option<Cow<'a, [T]>>, positions: Option<&Mapping>, m: usize, k: usize) -> Option<Cow<'a, [T]>> {
    b.map(|b| match positions { Some(p) => Cow::Owned((0..m).flat_map(|i| b[p.index(i) * k..][..k].to_vec()).collect()), None => b })
}

/// `kind •decompose⁻¹ factors` multiplies a record of factors from `•decompose` back into its matrix, taking `kind` as `•decompose` does. `"lu"`'s factors give back the matrix with its rows in their original order. Eigenvectors that aren't independent, such as a defective matrix's, can't determine the matrix. They raise a `DOMAIN` error. The factors multiply in the [`Domain`] of their items, or in floats of the default width where these are all exact. The result takes the widest float width among the factors.
pub(crate) fn recompose(crate::system::HostCall { left, right, numeric, .. }: crate::system::HostCall<'_>) -> Result<Value, Error> {
    let Decomposition { kind, vectors, .. } = Decomposition::new(left)?;
    if !vectors { return Err(Error::domain("•decompose⁻¹ needs the vectors")); }
    let names: &[(&str, usize)] = match kind.as_str() {
        "svd" => &[("u", 2), ("s", 1), ("v", 2)], "qr" => &[("q", 2), ("r", 2)], "cholesky" => &[("l", 2)], "eigen" => &[("values", 1), ("vectors", 2)],
        "lu" => &[("l", 2), ("u", 2), ("p", 1)], _ => unreachable!("`Decomposition::new` checks the kind")
    };
    // Each factor, with its rows and columns.
    let factors = names.iter().map(|&(name, rank)| {
        let f = crate::keyed::field(right, name).ok_or_else(|| Error::domain(format!("{kind} factors need {name}")))?;
        let size = match (f.shape(), rank) {
            (&[m, n], 2) => (m, n), (&[m], 1) => (m, 1), _ => return Err(Error::new(Kind::Rank, format!("{name} must have rank {rank}")))
        };
        Ok((f, size))
    }).collect::<Result<Vec<_>, Error>>()?;
    let arrays: Vec<&Value> = factors.iter().map(|(f, _)| f).collect();
    let (domain, width) = domain_of(&arrays, numeric)?;
    with_field!(domain.inexact(width), [Single, Double, Complex], T => {
        let factors = factors.iter().map(|(f, size)| Ok((read::<T>(f)?, *size))).collect::<Result<Vec<_>, Error>>()?;
        rebuild(&kind, &factors, width)
    })
}

/// The matrix that `kind`'s factors `f` multiply back into, as an array with real numbers rounded to `width`. Each factor is held row by row with its rows and columns, in the order `recompose` names them.
fn rebuild<T>(kind: &str, f: &[(Cow<'_, [T]>, (usize, usize))], width: FloatWidth) -> Result<Value, Error> where T: Factor + Field, PlusTimes: Inner<T, Out = T>
{
    let disagree = || Error::new(Kind::Length, format!("{kind} factors have sizes that don't agree"));
    let (x, size) = match (kind, f) {
        ("svd", [(u, (m, p)), (s, (ps, _)), (v, (n, pv))]) => {
            if p != ps || pv != ps { return Err(disagree()); }
            (linalg::from_svd(u, s, v, [*m, *p, *n]), [*m, *n])
        }
        ("qr", [(q, (m, p)), (r, (pr, n))]) => { if p != pr { return Err(disagree()); } (PlusTimes.inner(q, r, [*m, *p, *n]), [*m, *n]) }
        ("cholesky", [(l, (n, c))]) => { if n != c { return Err(disagree()); } (linalg::from_cholesky(l, *n), [*n, *n]) }
        ("eigen", [(values, (n, _)), (vectors, (r, c))]) => {
            if r != n || c != n { return Err(disagree()); }
            let a = linalg::from_eigen(values, vectors, *n).ok_or_else(|| Error::domain("eigenvectors that aren't independent can't give back the matrix"))?;
            (a, [*n, *n])
        }
        ("lu", [(l, (n, c)), (u, (nu, cu)), (p, (np, _))]) => {
            if [c, nu, cu, np].into_iter().any(|x| x != n) { return Err(disagree()); }
            (linalg::from_lu(l, u, p, *n).ok_or_else(|| Error::domain("lu's p must hold each position from 0 to n-1 once"))?, [*n, *n])
        }
        _ => unreachable!("`recompose` names the factors of each kind"),
    };
    matrix(x, size, width).described("invalid matrix")
}

pub(super) fn binary_encode(right: &Value) -> Result<Value, Error> {
    use num_traits::Signed;
    let invalid = || Error::domain("binary encoding needs nonnegative whole numbers");
    let mut numbers = Vec::with_capacity(right.len());
    let mut width = 0;
    for item in right.elements() {
        check()?;
        let n = numeric(&item)?;
        let value = n.big_integer().ok().filter(|v| !v.is_negative()).ok_or_else(invalid)?;
        width = width.max(saturated(value.bits()));
        numbers.push((value, n.clone()));
    }
    let layout = right.layout().concat(&Layout::from(vec![width]));
    let mut data = Vec::with_capacity(generated_len(layout.shape())?);
    for (n, domain) in &numbers {
        check()?;
        data.extend((0..width).rev().map(|bit| Value::Number(domain.like(i32::from(n.bit(bit as u64))))));
    }
    let prototype = Value::Number(numeric(&right.prototype())?.zero());
    layout.collect(data, prototype).described("invalid binary encoding")
}

/// The base that monadic `⊥` decodes in: binary, as a weak 2.
pub(crate) fn binary_base() -> Value { Value::weak_integer(2) }
/// Bases are vectors along the last axis of `left`. Its leading axes form a frame that agrees with the values' frame, as with J's ranks: 1 0 to encode and 1 1 to decode. Integer results take the width that fixed arguments give (`Numeric::for_values`), and a decoded value wraps there, as repeated `+` and `×` would wrap it. Static kinds wrap them at the `int` width.
pub(super) fn radix(left: &Value, right: &Value, encode: bool, numeric: Numeric) -> Result<Value, Error> {
    let numbers = |a: &Value| a.elements().map(|e| super::numeric(&e).cloned()).collect::<Result<Vec<_>, _>>();
    let zero = super::numeric(&right.prototype())?.result_zero(Some(super::numeric(&left.prototype())?));
    let settings = numeric.for_values([left, right])?;
    let error = |m| Error::domain(m);
    let leading = |a: &Value| a.layout().axes(0..a.shape().len().saturating_sub(1));
    let xlen = left.shape().last().copied().unwrap_or(1);
    let ylen = if encode { 1 } else { right.shape().last().copied().unwrap_or(1) };
    let agreement = Agreement::new(&leading(left), &if encode { right.layout().clone() } else { leading(right) }).described("radix frames do not agree")?;
    let cell = |m: &Mapping, i, len: usize| m.get(i).map(|j| j * len).ok_or_else(|| Error::domain("radix frames need matching keys"));
    if encode {
        let digits = if left.is_unit() { Layout::from(vec![]) } else { left.layout().axes(left.shape().len() - 1..left.shape().len()) };
        let layout = agreement.layout.concat(&digits);
        let len = generated_len(layout.shape())?;
        if let (Mapping::Single, Mapping::Linear(1)) = (&agreement.left, &agreement.right) {
            if let Some(result) = encode_whole(left.as_items(), right.as_items(), &layout) {
                return result.wrapped(settings).described("invalid encode result");
            }
        }
        let (xs, ys) = (numbers(left)?, numbers(right)?);
        let mut data = Vec::with_capacity(len);
        for i in 0..if xlen == 0 { 0 } else { agreement.len } {
            check()?;
            let numbers: Vec<&Number> = xs[cell(&agreement.left, i, xlen)?..][..xlen].iter().chain([&ys[cell(&agreement.right, i, 1)?]]).collect();
            let integers: Option<Vec<_>> = numbers.iter().map(|n| n.as_integer()).collect();
            let digits = if let Some(items) = integers {
                rustymath::integer::encode(&items[..xlen], &items[xlen..]).ok_or_else(|| error("an encode quotient is outside the machine domain"))?.into_iter()
                    .map(Number::from_integer).collect::<Vec<_>>()
            } else {
                with_field!(numeric; numbers.iter().copied(), T, width => {
                    let items: Vec<T> = numbers.iter().map(|&n| T::of(n)).collect();
                    rustymath::polynomial::encode(&items[..xlen], &items[xlen..], |b, v| b.residue(v, numeric, width)).into_iter().map(|d| d.number(width)).collect::<Vec<_>>()
                })
            };
            for digit in digits { data.push(Value::Number(digit.wrapped(settings))); }
        }
        if left.is_atom() && right.is_atom() { return Ok(data.remove(0)); }
        return layout.collect(data, Value::Number(zero)).described("invalid encode result");
    }
    let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), right.shape().len().saturating_sub(1))?;
    let len = rustymath::agreement::paired_length(xlen, ylen)?;
    // The general path gives a cell of one digit that digit unchanged, exact or not.
    if let (Mapping::Single, Mapping::Linear(1)) = (&agreement.left, &agreement.right) {
        if len > 1 && !agreement.layout.shape().is_empty() && (0..ylen).all(|k| positions.index(k) == k) {
            if let Some(result) = decode_compact(left, right, xlen, ylen, len, &agreement.layout) {
                return result.wrapped(settings).described("invalid decode result");
            }
        }
    }
    let (xs, ys) = (numbers(left)?, numbers(right)?);
    let mut data = Vec::with_capacity(agreement.len);
    for i in 0..agreement.len {
        check()?;
        let (x, y) = (&xs[cell(&agreement.left, i, xlen)?..], &ys[cell(&agreement.right, i, ylen)?..]);
        let digits: Vec<&Number> = (0..len).map(|k| &y[if ylen == 1 { 0 } else { positions.index(k) }]).collect();
        // A cell's first base is never used, so it takes no part in choosing the domain.
        let bases: Vec<&Number> = (1..len).map(|k| &x[if xlen == 1 { 0 } else { k }]).collect();
        data.push(Value::Number(match &digits[..] {
            [] => zero.clone(),
            [d] => (*d).clone(),
            ds => {
                let bases: Vec<_> = std::iter::once(ds[0]).chain(bases.iter().copied()).collect();
                let integers: Option<Vec<_>> = bases.iter().chain(ds).map(|n| n.as_integer()).collect();
                if let Some(integers) = integers {
                    Number::from_integer(rustymath::integer::decode_wrapped(&integers[..bases.len()], &integers[bases.len()..]).remove(0))
                } else { with_field!(numeric; bases.iter().chain(ds).copied(), T, width => {
                    rustymath::polynomial::decode(&bases.iter().map(|&n| T::of(n)).collect::<Vec<_>>(), &ds.iter().map(|&n| T::of(n)).collect::<Vec<_>>()).remove(0).number(width)
                }) }
            },
        }.wrapped(settings)));
    }
    if agreement.layout.shape().is_empty() { return Ok(data.remove(0)); }
    agreement.layout.collect(data, Value::Number(zero)).described("invalid decode result")
}

/// `bases⊤values` when both hold whole numbers in compact storage, with each value's digits together and in order. Each base divides by multiplication. The digits are floats when either argument is. `None` for other arguments, or when a quotient leaves `i64`.
fn encode_whole<'a>(bases: Items<'a>, values: Items<'a>, layout: &Layout) -> Option<Value> {
    let float = [bases, values].into_iter().filter_map(|items| if let Items::Compact(View::Floats(f)) = items { Some(f.tag()) } else { None }).max();
    let whole = |items: Items<'a>| match items { Items::Compact(View::Booleans(_) | View::Integers(_) | View::Floats(_)) => items.integers().ok(), _ => None };
    let (bases, values) = (whole(bases)?, whole(values)?);
    if bases.is_empty() { return None; }
    let data = rustymath::integer::encode(&bases, &values)?;
    match float { None => layout.integers(data), Some(width) => layout.floats(width, data.into_iter().map(rustymath::convert::float)) }.ok()
}

/// Compact radix decode. Integer steps wrap at i64; the caller applies the selected integer width. Float inputs retain their float width.
fn decode_compact(bases: &Value, digits: &Value, xlen: usize, ylen: usize, len: usize, layout: &Layout) -> Option<Value> {
    fn spread<T: Copy>(v: &[T], each: usize, len: usize) -> Cow<'_, [T]> {
        if each == len { Cow::Borrowed(v) } else { Cow::Owned(v.iter().flat_map(|&x| std::iter::repeat_n(x, len)).collect()) }
    }
    if let Some(width) = bases.float_width().max(digits.float_width()) {
        let (b, d) = (read::<f64>(bases).ok()?, read::<f64>(digits).ok()?);
        return layout.floats(width, rustymath::polynomial::decode(&spread(&b, xlen, len), &spread(&d, ylen, len))).ok();
    }
    let (b, d) = (bases.as_items().exact_integers()?, digits.as_items().exact_integers()?);
    let (b, d) = (spread(&b, xlen, len), spread(&d, ylen, len));
    layout.integers(rustymath::integer::decode_wrapped(&b, &d)).ok()
}

pub(crate) fn roles(call: &CallView<'_>) -> Result<Roles, Error> {
    if mixed_roots(call) { return Ok(Roles::default()); }
    let primitive = match call.operation {
        Operation::Primitive { primitive, .. } => Some(*primitive), Operation::Product { .. } => call.product_map().and_then(Function::dyadic_primitive),
        _ => None
    };
    let Some(primitive) = primitive else { return Ok(Roles::default()); };
    let roots: Vec<_> = call.layout().arguments().collect();
    let positions: Vec<_> = roots.iter().map(|&root| call.layout().start(root)).collect();
    let count = positions.len();
    if count == 0 || count > 2 { return Ok(Roles::default()); }
    let dyadic = count == 2;
    let circle = dyadic && matches!(primitive, Primitive::Math(Math::Circle | Math::Arc) | Primitive::RealCircle { .. });
    let mut roles = if circle { Roles::default().with(positions[0], ReadKind::Value) } else { Roles::default() };
    if !call.mode.dynamic { return Ok(roles); }
    let start = usize::from(circle);
    let mut types = [DType::Boolean; 2];
    let mut ranges = [rustymath::facts::Range::ANY; 2];
    for (to, &position) in positions[start..].iter().enumerate() {
        let Some(desc) = call.desc(position) else { return Ok(roles) };
        if matches!(desc.kind.dtype(), DType::Complex(_) | DType::Character) { return Ok(roles); }
        types[to] = desc.kind.dtype();
        ranges[to] = desc.bounds();
    }
    let code = if circle {
        call.desc(positions[0]).and_then(|desc| desc.range).and_then(|range| range.constant())
            .filter(|x| x.fract() == 0.0).map(|x| x as i64)
    } else { None };
    let variant = rustymath::facts::Variant { dtype: DType::Float(FloatWidth::F64), checked: true, wrapping: false, mathish: call.mode.mathish };
    let has_real_kind = match primitive.kernel(dyadic, &ObjectArgs { numeric: Numeric::EXACT, code, variant, types: &types[..count - start], ranges: &ranges[..count - start] }) {
        Ok(Some(kernel)) => matches!(kernel.result_dtype(), DType::Complex(_)) || kernel.domain(&ranges[..count - start]), _ => !circle
    };
    if !has_real_kind { for &position in &positions[start..] { roles = roles.with(position, ReadKind::Kind); } }
    Ok(roles)
}

pub(crate) type MathFactory = fn(Primitive, &ObjectArgs<'_>) -> Result<Option<Kernel>, Error>;

pub(crate) struct ObjectArgs<'a> {
    pub(super) numeric: Numeric,
    pub(super) code: Option<i64>,
    pub(super) variant: Variant,
    pub(super) types: &'a [DType],
    pub(super) ranges: &'a [Range],
}
impl ObjectArgs<'_> {
    pub(super) fn resolve<K: Formula<N>, const N: usize>(&self, object: K) -> Result<Option<Kernel>, Error> {
        let types = if let [kind] = self.types { [*kind; N] } else { self.types.try_into().map_err(|_| Error::domain("numeric operand count changed"))? };
        let ranges = if let [range] = self.ranges { [*range; N] } else { self.ranges.try_into().map_err(|_| Error::internal())? };
        let variant = if self.variant.dtype.is_integral() && !object.integer_domain(ranges) {
            Variant { dtype: DType::Float(self.numeric.exact_width(self.variant.dtype)), ..self.variant }
        } else { self.variant };
        object.resolve(variant, types).map(|kernel| Some(kernel.ranged(&ranges))).map_err(Into::into)
    }
    pub(super) fn tolerance(&self) -> tol::Tolerance {
        self.numeric.tolerance_at(match self.variant.dtype { DType::Float(width) => width, DType::Complex(width) => width.into(), _ => self.numeric.width })
    }
    pub(super) fn divide(&self, integral: bool) -> Result<Option<Kernel>, Error> {
        if integral {
            if self.variant.mathish { self.resolve::<_, 2>(tol::MathishFloorDivide(self.tolerance())) }
            else { self.resolve::<_, 2>(tol::FloorDivide(self.tolerance())) }
        } else if self.variant.mathish { self.resolve::<_, 2>(rm::MathishDivide) }
        else { self.resolve::<_, 2>(rm::Divide) }
    }
    pub(super) fn circle(&self, inverse: bool, real: bool) -> Result<Option<Kernel>, Error> {
        let Some(code) = self.code else { return Ok(None); };
        let kind = *self.types.last().ok_or_else(|| Error::domain("circle needs an argument"))?;
        Kernel::circle(code, inverse, real, self.variant, [kind]).map_err(Into::into)
    }
    pub(super) fn bitwise<const N: usize>(&self, primitive: Primitive) -> Result<Option<Kernel>, Error> where rm::Bitwise: Formula<N> {
        let Primitive::Bitwise(table) = primitive else { return Err(Error::internal()); };
        self.resolve::<_, N>(rm::Bitwise(table))
    }
}

pub(crate) fn map_arguments(call: &CallView<'_>) -> Result<(Option<usize>, usize), Error> {
    let mut positions = call.data_positions().filter(|position| call.layout().argument_positions().contains(position));
    let first = positions.next().ok_or_else(|| Error::domain("a numeric call needs an argument"))?;
    let pair = match positions.next() { None => (None, first), Some(second) => (Some(first), second) };
    if positions.next().is_some() { return Err(Error::domain("too many numeric arguments")); }
    Ok(pair)
}

fn map_variant(call: &CallView<'_>) -> Result<rustymath::facts::Variant, Error> {
    call.variant(call.numeric()?.kind(call.operand_kind()?.map_or(DType::Character, crate::number::Kind::dtype)))
}

fn mixed_roots(call: &CallView<'_>) -> bool {
    call.trees.is_some_and(|trees| call.layout().arguments().any(|root| matches!(trees.root_at(&call.layout().root(root)), Tree::Array { .. })))
}

/// Pervasive calls traverse their wrapper once and build the same numerical call at each native leaf.
pub(super) fn pervade_form(origin: &Origin<&[Value]>, call: &CallView) -> Result<Option<Outputs<Value>>, Error> {
    let roots = call.layout().arguments();
    if !mixed_roots(&call) || call.row().is_none_or(|row| !row.pervasive(roots.len())) { return Ok(None); }
    let Operation::Primitive { primitive, .. } = origin.operation else { return Ok(None); };
    let right = origin.root_input(roots.back().or_internal()?)?;
    let left = (roots.len() == 2).then(|| origin.root_input(roots.start)).transpose()?;
    let numeric = crate::settings::captured(&call)?;
    let agreement = left.as_deref().map(|left| map_agreement(&call, left.layout(), right.layout())).transpose()?;
    let fill = call.layout().fill().map(|root| origin.root_input(root.index)).transpose()?;
    let apply = |left: Option<&Value>, right: &Value| {
        let mut values: Vec<_> = left.into_iter().chain([right]).cloned().collect();
        let pairing = if let Some(fill) = fill.as_deref().filter(|_| values.iter().any(|value| !value.is_atom())) {
            values.extend([crate::keyed::empty_record(), fill.clone()]);
            Pairing::Explicit { fill: true }
        } else { Pairing::Axes };
        origin.apply(Operation::Primitive { primitive, pairing }, &values)
    };
    let prototype = |left: Option<&Value>, right: &Value| {
        let left = left.map(|value| value.read(ReadKind::Value)).transpose()?;
        let right = right.read(ReadKind::Value)?;
        let scalar = |left: Option<&Value>, right: &Value| scalar_map(&call, left, right, numeric, true);
        pervade(left.as_ref(), &right, numeric, None, fill.as_deref(), &Pervasion::atoms(&scalar, &scalar))
    };
    let leaf = |value: &Value| value.is_atom() || fill.as_deref().is_none_or(Value::is_atom) && Desc::of(value).is_some();
    let item = |value: &Value, position| {
        if value.is_atom() { return Ok(value.clone()); }
        if matches!(value, Value::Array(array) if matches!(&**array, crate::array::ArrayData::Mixed { .. })) { return Ok(value.at(position)); }
        let selected =
            crate::eval::projected_call(origin, Operation::Read(ReadOp::Cell { rank: 0 }), &[Value::integer(position as i64), value.clone()])?;
        Operation::dispatch(selected.reborrow())?.assemble()
    };
    let walk = Pervasion { apply: &apply, prototype: &prototype, leaf: &leaf, item: &item };
    pervade(left.as_deref(), &right, numeric, agreement, fill.as_deref(), &walk).map(|value| Some(Outputs::one(value)))
}
fn map_agreement(call: &CallView<'_>, left: &Layout, right: &Layout) -> Result<Agreement, Error> {
    match call.operation {
        Operation::Primitive { pairing: Pairing::Explicit { .. }, .. } => {
            let Some(root) = call.axis_root() else { return Ok(Agreement::new(left, right)?); };
            let axis = call.root_parameter(root.index)?;
            let axes = axes(&resolve_axes(&axis, right)?, right.shape().len())?;
            Ok(Agreement::with_axes(left, right, &axes)?)
        }
        Operation::Primitive { pairing: Pairing::Matrices { .. }, .. } => Err(Error::domain("elementwise calls do not use matrix pairing")),
        _ => Ok(Agreement::new(left, right)?),
    }
}
pub(crate) fn map_geometry(call: &CallView<'_>) -> Result<(Layout, bool, Option<Mapping>, Mapping), Error> {
    let (left, right) = map_arguments(call)?;
    let right = call.desc(right).or_internal()?;
    let Some(left) = left else { return Ok((right.layout.clone(), right.atom, None, Mapping::Linear(1))); };
    let left = call.desc(left).or_internal()?;
    let agreement = map_agreement(call, &left.layout, &right.layout)?;
    Ok((agreement.layout, left.atom && right.atom, Some(agreement.left), agreement.right))
}

pub(crate) fn result_kind(
    numeric: Numeric, mut dtype: DType, mut range: rustymath::facts::Range, policy: IntegerResult) -> (crate::number::Kind, rustymath::facts::Range) {
    let forced = policy.applies(!numeric.is_static()) && matches!(dtype, DType::Float(_));
    if forced {
        dtype = DType::Integer(if numeric.is_static() { numeric.int } else { Width::I64 });
        let bounds = rustymath::facts::Range::of_dtype(dtype);
        range = rustymath::facts::Range { lo: range.lo.clamp(bounds.lo, bounds.hi), hi: range.hi.clamp(bounds.lo, bounds.hi) };
    }
    let fixed = !forced && policy != IntegerResult::Always && numeric.fixed.is_some();
    if matches!(dtype, DType::Integer(_)) {
        let fits = |width| {
            let upper = 2f64.powi(DType::Integer(width).bytes() as i32 * 8 - i32::from(width != Width::U8));
            let lower = if width == Width::U8 { 0.0 } else { -upper };
            range.lo >= lower && range.hi < upper
        };
        dtype = if fixed || numeric.is_static() { DType::Integer(numeric.integer_width()) } else {
            DType::Integer([Width::U8, Width::I16, Width::I32, Width::I64].into_iter().find(|&width| fits(width)).unwrap_or(Width::I64))
        };
        if let DType::Integer(width) = dtype && !fits(width) { range = rustymath::facts::Range::of_dtype(dtype); }
    }
    (crate::number::Kind::stored(dtype, fixed, false), range)
}

fn result_variant(mut variant: rustymath::facts::Variant, result: DType) -> Result<rustymath::facts::Variant, Error> {
    if variant.dtype.is_integral() && result.is_integral() { variant.dtype = variant.dtype.common(result).or_internal()?; }
    Ok(variant)
}

/// Marks expand to the same numerical objects and conversion descriptor as formulas use.
pub(super) fn convert_form(origin: Origin<&[Value]>, call: &CallView<'_>) -> Result<Option<Outputs<Value>>, Error> {
    let Operation::Convert(mark) = origin.operation else { return Ok(None); };
    let root = call.layout().root(Root(0));
    if call.trees.is_some_and(|trees| !matches!(trees.root_at(&root), Tree::Leaf)) {
        return origin.map_root(Root(0), call, |value, _| origin.apply(Operation::Convert(mark), std::slice::from_ref(value))).map(Some);
    }
    let source = root.leaves.start;
    let Some(desc) = call.desc(source) else { return Ok(None); };
    let Some(kind) = mark.kind(desc.kind, call.numeric()?) else { return Ok(None); };
    let mut value = origin.list[source].clone();
    if mark == Mark::Real && matches!(desc.dtype(), DType::Complex(_)) {
        value = origin.apply(Operation::Primitive { primitive: Primitive::Special(Special::Real), pairing: Pairing::Axes }, &[value])?;
    }
    if matches!(kind.dtype(), DType::Integer(_)) && matches!(desc.dtype(), DType::Float(_)) {
        value = origin.apply(Operation::Recipe(RecipeOp::FloatFloor), &[value])?;
    }
    use rustymath::convert::ConversionKind::{Cast, Truncate};
    let conversion = if matches!(kind.dtype(), DType::Integer(_)) { Truncate } else { Cast };
    let converted = crate::eval::projected_call(&origin, Operation::Recipe(RecipeOp::Convert { kind, conversion }), &[value])?;
    Operation::dispatch(converted.reborrow()).map(Some)
}

pub(super) fn code_convert(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    let Operation::Convert(mark) = call.operation else { return Err(Error::internal()); };
    let value = call.root_value(Root(0), ReadKind::Value)?;
    let numeric = call.numeric()?;
    let convert = |_: Option<&Value>, value: &Value| mark.number(super::numeric(value)?, numeric).map(Value::Number).map_err(Error::from);
    Ok(Outputs::one(pervade(None, &value, numeric, None, None, &Pervasion::atoms(&convert, &convert))?))
}
pub(super) fn build_map(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    if call.row().is_some_and(|row| !row.pervasive(call.layout().arguments().len())) { return host_build(call, set); }
    let (left, right) = map_arguments(call)?;
    if call.desc(right).is_none() || left.is_some_and(|left| call.desc(left).is_none()) { return host_build(call, set); }
    if let Operation::Recipe(operation) = call.operation
        && let Some((conversion, kind)) = operation.conversion(call.desc(right).expect("a native conversion has a description").dtype()) {
            let source = call.desc(right).expect("a native conversion has a description");
            if matches!(operation, RecipeOp::Convert { conversion: rustymath::convert::ConversionKind::Keep, .. }) && !source.atom {
                return Err(Error::domain("a typed array needs an atom fill"));
            }
            let desc = Desc { kind, atom: source.atom, layout: source.layout.clone(), range: None };
            set(right, Mapping::Linear(1));
            return Ok((Step::Convert { arg: right as u32, conversion }, Some(Outputs::one(Some(desc))), None));
        }
    let variant = map_variant(call)?;
    let Some(kernel) = call.kernel(variant)? else { return host_build(call, set); };
    let (layout, atom, left_map, right_map) = map_geometry(call)?;
    let mut ranges = [rustymath::facts::Range::ANY; 2];
    let mut count = 0;
    for position in left.into_iter().chain([right]) {
        ranges[count] = call.desc(position).expect("numeric arguments have descriptions").bounds();
        count += 1;
    }
    if call.mode.dynamic && !kernel.domain(&ranges[..count]) { return host_build(call, set); }
    let integer = call.row().map_or(IntegerResult::Never, |row| row.integer_result[usize::from(left.is_some())]);
    let (kind, range) = result_kind(call.numeric()?, kernel.result_dtype(), kernel.range(&ranges[..count]), integer);
    let variant = result_variant(variant, kind.dtype())?;
    let step = Step::Map { left: left.map(|position| position as u32), right: right as u32, variant };
    if let Some(left) = left { set(left, left_map.expect("a pair has two maps")); }
    set(right, right_map);
    let desc = Desc { kind, atom, layout, range: Some(range) };
    Ok((step, Some(Outputs::one(Some(desc))), None))
}

pub(super) fn code_recipe(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    let Operation::Recipe(operation) = call.operation else { return Err(Error::internal()); };
    let (_, right) = map_arguments(call)?;
    if let RecipeOp::Convert { kind, conversion: rustymath::convert::ConversionKind::Keep } = operation {
        return keep_atom(call.value(right, ReadKind::Value)?.as_ref(), *kind).map(Outputs::one);
    }
    let Some(dtype) = call.desc(right).map(|desc| desc.dtype()) else { return code_map(call); };
    let Some((conversion, kind)) = operation.conversion(dtype) else { return code_map(call); };
    converted_value(call.value(right, ReadKind::Value)?.as_ref(), conversion, kind, call.mode.dynamic).map(Outputs::one)
}

fn converted_value(value: &Value, conversion: rustymath::convert::Conversion, kind: NumberKind, checked: bool) -> Result<Value, Error> {
    use rustymath::{agreement::{Binding, Read}, buffer::Buffer};
    let (view, plan) = value.physical_read(0).ok_or_else(|| Error::domain("a native conversion needs native data"))?;
    let sources = [Binding::full(view)];
    let mut buffer = Buffer::with_capacity(kind.dtype(), plan.len());
    conversion.map(Read { sources: &sources, plan: &plan }, 0..plan.len(), checked, &mut buffer)?;
    Value::stored(&Desc::new(kind, value.is_atom(), value.layout().clone()), buffer)
}

/// A typed fill uses the same Keep conversion, including an explicit unbounded scalar.
pub(crate) fn keep_atom(value: &Value, kind: NumberKind) -> Result<Value, Error> {
    let lost = || Error::domain("the fill doesn't keep its value as the data's type");
    if !value.is_atom() { return Err(lost()); }
    let value = match value {
        Value::Number(number) if number.is_unbounded() => Cow::Owned(Value::Number(match kind.dtype() {
            DType::Boolean | DType::Integer(_) => Number::from_integer(number.integer().map_err(|_| lost())?),
            DType::Float(_) | DType::Complex(_) => Number::float(number.to_float().map_err(|_| lost())?, FloatWidth::F64),
            DType::Character => return Err(lost()),
        })),
        value => Cow::Borrowed(value),
    };
    let conversion =
        rustymath::convert::Conversion { input: value.dtype().ok_or_else(lost)?, output: kind.dtype(), kind: rustymath::convert::ConversionKind::Keep };
    converted_value(&value, conversion, kind, true).map_err(|_| lost())
}

pub(super) fn definition(origin: &Origin<&[Value]>, call: &CallView<'_>) -> Result<Outputs<Value>, Error> {
    use crate::number::{Arithmetic::*, Math::*};
    let Operation::Primitive { primitive, .. } = call.operation else { unreachable!("a primitive definition") };
    let root = call.layout().arguments().last().ok_or_else(|| Error::domain("a function needs an argument"))?;
    let right = origin.root_input(root)?.into_owned();
    let apply = |primitive, args: &[Value]| {
        origin.apply(Operation::Primitive { primitive, pairing: Pairing::Axes }, args).map(Outputs::one)
    };
    match primitive {
        Primitive::Math(Nand) => apply(Primitive::Arithmetic(Times), &[right.clone(), right.clone()]),
        Primitive::Math(Nor) => apply(Primitive::Arithmetic(Plus), &[right.clone(), right.clone()]),
        Primitive::Math(Gcd | Lcm) => complex_parts(origin, call, *primitive == Primitive::Math(Lcm)),
        Primitive::Compare(Comparison::LessEqual | Comparison::GreaterEqual) => {
            let op = if *primitive == Primitive::Compare(Comparison::LessEqual) { Minus } else { Plus };
            apply(Primitive::Arithmetic(op), &[right.clone(), Value::weak_integer(1)])
        }
        _ => unreachable!("the row declares its semantic definition"),
    }
}
pub(super) fn code(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    let Operation::Primitive { primitive, .. } = call.operation else { unreachable!("a primitive row") };
    match (primitive, call.layout().arguments().len() == 2) {
        (Primitive::Compare(_), false) | (Primitive::Math(Math::Not), true) => super::host_primitive(call), _ => code_map(call),
    }
}

pub(crate) fn scalar(primitive: Primitive, left: Option<&Number>, right: &Number, numeric: Numeric) -> Result<Number, Error> {
    let args: Vec<_> = left.into_iter().chain([right]).map(|number| Value::Number(number.clone())).collect();
    let origin = Operation::Primitive { primitive, pairing: Pairing::Axes }.origin_numeric(&args, numeric)?;
    let current = origin.reborrow();
    let prepared = Operation::prepare(&current)?;
    let value = scalar_map(&prepared.view(), left.map(|_| &args[0]), args.last().expect("a scalar argument"), numeric, false)?;
    value.as_number().ok_or_else(|| Error::domain("a scalar formula returned an array"))
}

fn scalar_map(call: &CallView<'_>, left: Option<&Value>, right: &Value, numeric: Numeric, prototype: bool) -> Result<Value, Error> {
    let args: Vec<_> = left.into_iter().chain([right]).collect();
    let kinds: Vec<_> = args.iter().filter_map(|value| value.kind()).filter(|kind| kind.dtype() != DType::Character).collect();
    let numeric = numeric.for_kinds(kinds.iter().copied()).map_err(Error::domain)?;
    let kind = kinds.into_iter().reduce(|left, right| left.join(right).expect("joined numeric kinds"));
    let dtype = kind.map_or(DType::Float(numeric.width), |kind| numeric.kind(kind.dtype()));
    let variant = rustymath::facts::Variant { dtype, checked: call.mode.dynamic, wrapping: dtype.is_integral(),
        mathish: numeric.mathish_at(crate::array::float_width_of(dtype)) };
    let types: Vec<_> = args.iter().map(|value| value.dtype().unwrap_or(DType::Float(numeric.width))).collect();
    let ranges: Vec<_> = args.iter().map(|value| Desc::of(value).map_or(rustymath::facts::Range::ANY, |desc| desc.bounds())).collect();
    let resolve = |variant| {
        let args = ObjectArgs { numeric, code: left.and_then(Value::as_number).and_then(|number| number.integer().ok()), variant, types: &types, ranges: &ranges };
        match call.operation {
            Operation::Recipe(RecipeOp::FloatFloor) => Primitive::Math(Math::Floor).kernel(false, &args),
            Operation::Recipe(operation) => Ok(operation.kernel(variant)),
            Operation::Primitive { primitive, .. } => primitive.kernel(call.layout().arguments().len() == 2, &args),
            _ => unreachable!("a mapped operation")
        }
    };
    let mut kernel = resolve(variant)?.ok_or_else(|| Error::domain("numeric formula does not take these kinds"))?;
    if !prototype && call.mode.dynamic && !kernel.domain(&ranges) && !matches!(dtype, DType::Complex(_) | DType::Character) {
        let width = kind.map_or(numeric.width, |kind| kind.real_width(numeric));
        if let Ok(Some(complex)) = resolve(rustymath::facts::Variant { dtype: DType::Float(width).complex(), ..variant }) { kernel = complex; }
    }
    let policy = call.row().map_or(IntegerResult::Never, |row| row.integer_result[usize::from(left.is_some())]);
    let (kind, _) = result_kind(numeric, kernel.result_dtype(), kernel.range(&ranges), policy);
    if kernel.variant.dtype.is_integral() && kind.dtype().is_integral() {
        let dtype = kernel.variant.dtype.common(kind.dtype()).or_internal()?;
        if dtype != kernel.variant.dtype { kernel = resolve(rustymath::facts::Variant { dtype, ..kernel.variant })?.or_internal()?; }
    }
    if prototype {
        if policy != IntegerResult::Always && kernel.has_exact() && args.iter().any(|value| value.as_number().is_some_and(|number| number.is_unbounded()))
            && args.iter().all(|value| value.as_number().is_some_and(|number| number.is_exact())) {
                return Ok(Value::Number(if kernel.result_dtype() == DType::Boolean { Number::from_bool(false) }
                else { right.as_number().expect("numeric prototype").result_zero(left.and_then(Value::as_number).as_ref()) }));
            }
        return Ok(Desc::new(kind, true, Layout::default()).zero());
    }
    let value = crate::number::calculate(&kernel, &args, numeric)?;
    if let Value::Number(number) = &value && (policy == IntegerResult::Always || policy.applies(call.mode.dynamic) && number.as_float().is_some()) {
        return Ok(Value::Number(number.whole(numeric)?));
    }
    Ok(value)
}

fn code_map(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    let numeric = call.numeric()?;
    let (left, right) = map_arguments(call)?;
    let right = call.value(right, ReadKind::Value)?;
    let left = left.map(|position| call.value(position, ReadKind::Value)).transpose()?;
    let agreement = left.as_ref().map(|left| map_agreement(call, left.layout(), right.layout())).transpose()?;
    let fill = call.layout().fill().map(|root| call.root_value(root.index, ReadKind::Value)).transpose()?;
    let apply = |left: Option<&Value>, right: &Value| scalar_map(call, left, right, numeric, false);
    let prototype = |left: Option<&Value>, right: &Value| scalar_map(call, left, right, numeric, true);
    let walk = Pervasion::atoms(&apply, &prototype);
    Ok(Outputs::one(pervade(left.as_deref(), &right, numeric, agreement, fill.as_deref(), &walk)?))
}

pub(crate) fn product_kernels(
    call: &CallView<'_>, result: Option<DType>) -> Result<Option<(rustymath::operation::Kernel, Option<rustymath::operation::Kernel>)>, Error> {
    let functions = call.operation.product_functions().or_internal()?;
    let Some(map) = call.product_map().and_then(Function::dyadic_primitive) else { return Ok(None); };
    let numeric = call.numeric()?;
    let mut variant = map_variant(call)?;
    let Some(mut mapper) = call.kernel_for(map, true, variant)? else { return Ok(None); };
    let arguments = call.layout().argument_positions();
    let ranges: Vec<_> = arguments.clone().filter_map(|position| call.desc(position).map(|desc| desc.bounds())).collect();
    if ranges.len() != arguments.len() { return Ok(None); }
    let (kind, _) = result_kind(numeric, mapper.result_dtype(), mapper.range(&ranges), IntegerResult::Never);
    if variant.dtype.is_integral() && kind.dtype().is_integral() {
        variant.dtype = variant.dtype.common(kind.dtype()).or_internal()?;
        let Some(widened) = call.kernel_for(map, true, variant)? else { return Ok(None); };
        mapper = widened;
    }
    let reducer = if functions.fold.is_some() {
        let Some(fold) = call.product_fold().and_then(Function::dyadic_primitive) else { return Ok(None); };
        let dtype = mapper.result_dtype();
        let working = match result {
            Some(result) if dtype.is_integral() && result.is_integral() => dtype.common(result).or_internal()?, _ => dtype
        };
        let Some(reducer) = fold.kernel(true, &ObjectArgs { numeric, code: None, variant: call.variant(working)?, types: &[dtype; 2], ranges: &[Range::ANY; 2] })? else {
            return Ok(None);
        };
        Some(reducer)
    } else { None };
    Ok(Some((mapper, reducer)))
}

fn fold_positions(call: &CallView<'_>, geometry: &crate::plan::Fold) -> Result<(Desc, bool, usize), Error> {
    use rustymath::reduce::Reduce;
    let max = match call.kernel(map_variant(call)?)?.and_then(|kernel| kernel.fold()) {
        Some(Reduce::Max) => true, Some(Reduce::Min) => false, _ => return Err(Error::domain("winner positions need a minimum or maximum fold"))
    };
    if geometry.seed.is_some() { return Err(Error::domain("winner positions need an unseeded fold")); }
    let Operation::Fold { positions: Some(frame), .. } = call.operation else { return Err(Error::internal()); };
    let size = geometry.source.shape().get(*frame..).or_internal()?.iter().product::<usize>();
    let kind = NumberKind::stored(DType::Integer(Width::I64), false, false);
    let range = rustymath::facts::Range { lo: 0.0, hi: size.saturating_sub(1) as f64 };
    Ok((Desc { kind, atom: geometry.atom, layout: geometry.result.clone(), range: Some(range) }, max, size))
}

pub(super) fn code_fold_positions(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    let geometry = crate::plan::fold(call)?;
    let (desc, max, size) = fold_positions(call, &geometry)?;
    let value = call.value(geometry.arg, ReadKind::Value)?;
    let dims = geometry.dims();
    let [_, n, inner] = dims;
    let of = if geometry.scan { rustymath::reduce::Picks::Running } else { rustymath::reduce::Picks::Lanes };
    let rows = value.view().and_then(|view| rustymath::reduce::view_picks(view, dims, of, max))
        .ok_or_else(|| Error::domain("winner positions need a real numeric array"))?;
    let offsets = rows.into_iter().enumerate().map(|(j, row)| {
        let outer = if geometry.scan { j / (n * inner) } else { j / inner };
        (((outer * n + row) * inner + j % inner) % size.max(1)) as i64
    }).collect();
    Ok(Outputs::one(Value::stored(&desc, Buffer::Integers(IntBuf::I64(offsets)))?))
}
pub(crate) fn build_fold(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    let positions = matches!(call.operation, Operation::Fold { positions: Some(_), .. });
    let roots = call.layout().arguments();
    if roots.clone().any(|root| call.root_desc(root).is_none()) || roots.len() == 2 && call.root_atom(roots.start) != Some(true) {
        if positions { return Err(Error::domain("winner positions need a real numeric array")); }
        return host_build(call, set);
    }
    let geometry = crate::plan::fold(call)?;
    if positions {
        let (desc, _, _) = fold_positions(call, &geometry)?;
        set(geometry.arg, geometry.input_read(call, geometry.arg)?);
        return Ok((Step::Call, Some(Outputs::one(Some(desc))), None));
    }
    let source = call.desc(geometry.arg).expect("a native fold has a source description");
    let length = geometry.dims()[1];
    if source.dtype() == DType::Character && (geometry.result.count() == 0 || geometry.seed.is_none() && length == 1) {
        let read = ReadPlan::copy(geometry.arg, source.layout.count(), Mapping::Linear(1), geometry.result.count(), Missing::Default);
        let desc = Desc { layout: geometry.result, atom: geometry.atom, ..source.into_owned() };
        set(geometry.arg, Mapping::Linear(1));
        return Ok((Step::Copy { source: geometry.arg as u32 }, Some(Outputs::one(Some(desc))), Some(read)));
    }
    let variant = map_variant(call)?;
    let Some(kernel) = call.kernel(variant)? else { return host_build(call, set); };
    let items = source.bounds();
    let seed = geometry.seed.and_then(|seed| call.desc(seed)).map(|seed| seed.bounds());
    let empty_identity = !geometry.scan && length == 0 && seed.is_none() && kernel.identity().is_some_and(|n| !n.is_finite());
    let mut range = if length == 0 { seed.unwrap_or_else(|| rustymath::facts::Range::of(kernel.identity())) }
    else { kernel.fold_range(items, seed, length) };
    if geometry.scan { range = range.join(items); }
    let dtype = if empty_identity { DType::Float(call.numeric()?.width_of([source.kind.float_width()])) }
    else if geometry.scan { kernel.scan_dtype() }
    else { kernel.fold_dtype(source.dtype(), length, geometry.seed.and_then(|seed| call.desc(seed)).map(|seed| seed.dtype())) };
    let (kind, range) = result_kind(call.numeric()?, dtype, range, IntegerResult::Never);
    let variant = if empty_identity { call.variant(kind.dtype())? } else { result_variant(variant, kind.dtype())? };
    for input in geometry.seed.into_iter().chain([geometry.arg]) { set(input, geometry.input_read(call, input)?); }
    let step = Step::Fold { arg: geometry.arg as u32, seed: geometry.seed.map(|seed| seed as u32), axis: geometry.axis, variant };
    let desc = Desc { kind, atom: geometry.atom, layout: geometry.result, range: Some(range) };
    Ok((step, Some(Outputs::one(Some(desc))), None))
}

pub(crate) fn build_product(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    if call.layout().arguments().any(|root| call.root_desc(root).is_none()) { return host_build(call, set); }
    let geometry = crate::plan::product(call)?;
    let Some((map, fold)) = product_kernels(call, None)? else { return host_build(call, set); };
    let ranges = geometry.args.map(|position| call.desc(position).expect("a native product has input descriptions").bounds());
    if call.mode.dynamic && !map.domain(&ranges) { return host_build(call, set); }
    let (range, dtype) = if let Some(fold) = &fold {
        let shape = geometry.plan.iteration.shape();
        let count = shape[shape.len() - geometry.plan.contracted..].iter().product();
        (fold.fold_range(map.range(&ranges), None, count), fold.fold_dtype(map.result_dtype(), count, None))
    } else { (map.range(&ranges), map.result_dtype()) };
    let mut numeric = call.numeric()?;
    if map.result_dtype() == DType::Boolean { numeric.fixed = None; }
    let (kind, range) = result_kind(numeric, dtype, range, IntegerResult::Never);
    let atom = geometry.plan.layout.shape().is_empty();
    let step = if fold.is_some() { Step::Product { left: geometry.args[0] as u32, right: geometry.args[1] as u32 } } else {
        Step::Map { left: Some(geometry.args[0] as u32), right: geometry.args[1] as u32, variant: result_variant(map_variant(call)?, kind.dtype())? }
    };
    for (position, map) in geometry.args.into_iter().zip(geometry.plan.mappings) { set(position, map); }
    let desc = Desc { kind, atom, layout: geometry.plan.layout.into(), range: Some(range) };
    Ok((step, Some(Outputs::one(Some(desc))), None))
}
