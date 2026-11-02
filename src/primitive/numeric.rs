//! Numeric primitives outside the pervasive kernels: matrix division and decompositions, encode and decode, complex parts, number formatting, the Lambert W function and random numbers.

use super::*;
use crate::execution::check;
use crate::number::{Domain, Field, read, with_field};
use num_complex::Complex64;
use num_rational::BigRational;
use rustymath::agreement::Frames;
use rustymath::array::ArrayView;
use rustymath::buffer::FloatWidth;
use rustymath::linalg::{self, Factor, Inner, PlusTimes};
use rustymath::{buffer::View, error::Kind, with_complexes};

pub(super) fn complex_parts(call: &CallView<'_>, polar: bool) -> Result<Outputs<Value>, Error> {
    let source = call.explicit_arguments().last().expect("a parts argument");
    let right = call.input(source).expect("a parts argument").value;
    let convert = polar && right.kind().is_some_and(|kind| kind.is_exact() && (!call.mode.dynamic || kind.fixed_width().is_some()));
    let right = if convert { Cow::Owned(call.apply(Operation::Convert(Mark::Real), std::slice::from_ref(right), &[Some(source)])?) } else { Cow::Borrowed(right) };
    let part = |primitive| call.apply(Operation::Primitive { primitive, pairing: Pairing::Axes }, std::slice::from_ref(right.as_ref()), &[(!convert).then_some(source)]);
    let (left, right) = if polar {
        (part(Primitive::Math(Math::Magnitude))?, part(Primitive::Special(Special::Angle))?)
    } else { (part(Primitive::Special(Special::Real))?, part(Primitive::Special(Special::Imag))?) };
    let column = |value: Value| {
        let axes = (0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }).chain([ViewAxis { input: 0, axis: None }]).collect();
        let destinations = (0..value.rank()).collect();
        call.apply(Operation::Read(ReadOp::Axes { axes, destinations }), &[value], &[None])
    };
    let value = call.apply(Operation::Primitive { primitive: Primitive::Ravel, pairing: Pairing::Axes }, &[column(left)?, column(right)?], &[None, None])?;
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
    crate::transform::formats(left, right)?;
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

pub(crate) fn lambert_w(right: &Value, span: &Span) -> Result<Value, Error> {
    Primitive::Special(Special::LambertW).call(None, right, span)
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
        let x = inexact(&args, (domain, width), |s, v| match *v {
            [a, b] => s.solve(a, b), [a] => s.pinv(a), _ => unreachable!("a view for each argument")
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

/// Monadic `-.×` on the `n`×`n` matrix `a`: its determinant. Exact numbers stay exact, through row reduction, and an integer determinant wraps where the call wraps (`Number::wrapped`). Floats and complex numbers take the LU determinant of rustymath's [`Stacked`](linalg::Stacked), as `⌹` reads them. `None` where an item isn't a number.
pub(crate) fn determinant(a: &Value, n: usize, numeric: Numeric) -> Result<Option<Value>, Error> {
    let Some((domain, width)) = Domain::of_arrays(&[a], numeric) else { return Ok(None) };
    if domain == Domain::Exact {
        let settings = numeric.for_values([a])?;
        return Ok(Some(Value::Number(linalg::exact::determinant(&read::<BigRational>(a)?, n)?.number(width).wrapped(settings))));
    }
    let d = inexact(&[(a, Layout::from([n, n]))], (domain, width), |s, v| s.det(v[0])).map_err(|s| s.raised())?;
    Ok(Some(value(d.into_compact().1, Layout::default())?.at(0)))
}

/// `⌹a` and the determinant of `a`, from one factorization, for a square matrix or a number `a`, or for each matrix in the last two axes of `a` where `stacked`, as `∂`'s rules for `⌹` and `-.×` need them. A stack's determinants keep its frame's axes. `None` where `⌹` gives the pseudoinverse of some matrix: a singular exact matrix, or a float matrix with a negligible LU pivot. Floats and complex numbers go through rustymath's [`Stacked`](linalg::Stacked), as `⌹` reads them. Exact matrices factor on rustymath's worker threads, which check the running evaluation's stops within each.
pub(crate) fn matrix_inverse(a: &Value, stacked: bool, numeric: Numeric) -> Result<Option<(Value, Value)>, Error> {
    let r = a.shape().len();
    let (frame, n) = match *a.shape() { [] | [1] => (0, 1), [.., m, n] if m == n && (stacked || r == 2) => (r - 2, n), _ => return Ok(None) };
    let (domain, width) = domain_of(&[a])?;
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
    let Some((x, d)) = inexact(&[(a, matrices)], (domain, width), |s, v| s.inverse(v[0])).map_err(|s| s.raised())? else { return Ok(None) };
    let (x, d) = (value(x.into_compact().1, inverse)?, value(d.into_compact().1, determinant)?);
    Ok(Some((if a.is_atom() { x.at(0) } else { x }, if frame == 0 { d.at(0) } else { d })))
}

/// A result of `⌹`, `x`, laid out by `layout`, or its one item where it's `scalar`.
fn matrix_result<T: Field>(mut x: Vec<T>, layout: Layout, scalar: bool, width: FloatWidth) -> Result<Value, Error> {
    if scalar { return Ok(Value::Number(x.swap_remove(0).number(width))); }
    T::array(x, layout.shape().to_vec(), width).and_then(|v| v.with_layout(layout)).described("invalid matrix result")
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
    }.and_then(|v| v.with_layout(layout)).described("invalid matrix result")
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

/// `kind •decompose⍤2` on a stack of matrices with compact storage and a plain layout: the array of factors that `⍤2` mixes from each cell's record, as `•decompose` gives it for that cell alone. A stack with failed cells gives an error. `None` where `•decompose⍤2` takes its general path, which also raises the errors that the options and the cells' shape give.
pub(crate) fn stacked_decompositions(left: &Value, right: &Value, span: &Span, numeric: Numeric) -> Option<Result<Value, Error>> {
    if right.dtype().is_none() || !right.layout().plain() || right.shape().len() < 3 || right.is_empty() { return None; }
    let d = Decomposition::new(Some(left)).ok()?;
    let [m, n] = matrix_size(right)?;
    if d.square() && m != n { return None; }
    Some(factors(&d, right, None, span, numeric).and_then(|(names, cells)| {
        let keys = record(names.clone(), cells[..names.len()].to_vec())?.layout().clone();
        let layout = Layout::from(right.shape()[..right.shape().len() - 2].to_vec()).concat(&keys);
        Value::new(layout.shape().to_vec(), cells).and_then(|v| v.with_layout(layout)).described("invalid rank cells")
    }))
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
        let a = v[0];
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
        let stack = Value::assemble(&[chosen.len()], &chosen, &chosen[0], crate::array::Widening::Items).described("invalid matrices")?;
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

/// The [`Domain`] that the items of `arrays` compute in for a matrix function, whose results are approximate (`Domain::approximate`), and the float width of the results. DOMAIN where an item isn't a number.
fn domain_of(arrays: &[&Value], numeric: Numeric) -> Result<(Domain, FloatWidth), Error> { Domain::approximate(arrays, numeric).ok_or_else(|| Error::domain("expected numeric elements")) }

/// The rows and columns of each matrix in `a`'s last two axes. `None` where `a` has fewer than two axes.
fn matrix_size(a: &Value) -> Option<[usize; 2]> { match *a.shape() { [.., m, n] => Some([m, n]), _ => None } }

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
    let numbers = |a: &Value| a.elements().map(|e| numeric(&e).cloned()).collect::<Result<Vec<_>, _>>();
    let zero = numeric(&right.prototype())?.result_zero(Some(numeric(&left.prototype())?));
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
        let t = numeric;
        let width = t.width_of(xs.iter().chain(&ys).map(Number::float_width));
        let mut data = Vec::with_capacity(len);
        for i in 0..if xlen == 0 { 0 } else { agreement.len } {
            check()?;
            let numbers: Vec<&Number> = xs[cell(&agreement.left, i, xlen)?..][..xlen].iter().chain([&ys[cell(&agreement.right, i, 1)?]]).collect();
            // Whole numbers encode exactly, floats among them too. Their digits are then floats of `width` where any of them is a float.
            let whole: Option<Vec<Number>> = numbers.iter().map(|n| n.big_integer().ok().map(Number::from)).collect();
            let inexact = whole.is_some() && !numbers.iter().all(|n| n.is_exact());
            let items = whole.unwrap_or_else(|| numbers.into_iter().cloned().collect());
            let digits = with_field!(numeric; items.iter(), T, w => {
                let items: Vec<T> = items.iter().map(T::of).collect();
                rustymath::polynomial::encode(&items[..xlen], &items[xlen..], |b, v| b.residue(v, t, w)).into_iter().map(|d| d.number(w)).collect::<Vec<_>>()
            });
            for d in digits { data.push(Value::Number(if inexact { d.to_float_width(width).map_err(error)? } else { d.wrapped(settings) })); }
        }
        if left.is_atom() && right.is_atom() { return Ok(data.remove(0)); }
        return layout.collect(data, Value::Number(zero)).described("invalid encode result");
    }
    let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), right.shape().len().saturating_sub(1))?;
    let len = rustymath::agreement::paired_length(xlen, ylen)?;
    // The general path gives a cell of one digit that digit unchanged, exact or not.
    if let (Mapping::Single, Mapping::Linear(1)) = (&agreement.left, &agreement.right) {
        if len > 1 && !agreement.layout.shape().is_empty() && (0..ylen).all(|k| positions.index(k) == k) {
            if let Some(result) = decode_compact(left, right, xlen, ylen, len, &agreement.layout, settings.wrapping()) {
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
        data.push(Value::Number(
            match &digits[..] {
                [] => zero.clone(),
                [d] => (*d).clone(),
                ds => with_field!(numeric; bases.iter().chain(ds).copied(), T, width => {
                    // The first digit stands in for the first base, in the domain that the other numbers chose.
                    let bases: Vec<T> = std::iter::once(ds[0]).chain(bases.iter().copied()).map(T::of).collect();
                    rustymath::polynomial::decode(&bases, &ds.iter().map(|&n| T::of(n)).collect::<Vec<_>>()).remove(0).number(width)
                }),
            }.wrapped(settings),
        ));
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

/// `bases⊥digits` when both hold numbers in compact storage, with each cell of `len` digits together and in order. `bases` holds `xlen` bases and each cell `ylen` digits, where 1 serves every position. The value is floats when either argument holds floats, and exact when both hold integers. Where integers wrap, as `wraps` says, each step wraps at 64 bits, and the caller wraps the values to the call's width. `None` for other arguments, or when a step of integers that don't wrap leaves `i64`.
fn decode_compact(bases: &Value, digits: &Value, xlen: usize, ylen: usize, len: usize, layout: &Layout, wraps: bool) -> Option<Value> {
    fn spread<T: Copy>(v: &[T], each: usize, len: usize) -> Cow<'_, [T]> {
        if each == len { Cow::Borrowed(v) } else { Cow::Owned(v.iter().flat_map(|&x| std::iter::repeat_n(x, len)).collect()) }
    }
    if let Some(width) = bases.float_width().max(digits.float_width()) {
        let (b, d) = (crate::element::read_as::<f64>(bases)?, crate::element::read_as::<f64>(digits)?);
        return layout.floats(width, rustymath::polynomial::decode(&spread(&b, xlen, len), &spread(&d, ylen, len))).ok();
    }
    let (b, d) = (bases.as_items().exact_integers()?, digits.as_items().exact_integers()?);
    let (b, d) = (spread(&b, xlen, len), spread(&d, ylen, len));
    layout.integers(if wraps { rustymath::integer::decode_wrapped(&b, &d) } else { rustymath::integer::decode(&b, &d)? }).ok()
}

/// `¿`, drawing from `generator`, or from the session's generator where `generator` is `None`: a deal where `left` is given, and otherwise a roll of each item of `right`. Each is a draw node (`distribution::drawn`, target part 17), which draws every cell of a running batched map.
pub(crate) fn drawn_value(generator: Option<&crate::distribution::Generator>, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    match left {
        None => crate::distribution::drawn(generator, crate::distribution::Law::Roll, &[right], span),
        Some(left) => crate::distribution::drawn(generator, crate::distribution::Law::Deal, &[left, right], span),
    }
}

/// Roll every item of `right`, keeping its layout. Each result is exact when its bound is.
pub(crate) fn roll_array(right: &Value, items: &mut rustymath::random::Items, span: &Span) -> Result<Value, Error> {
    roll(right, span, false, items)?.with_layout(right.layout().clone()).described("invalid roll result")
}

/// One number below `n`, as `Numbers::number` gives it, or a float of width `unit` between 0 and 1 when `n` is 0, from the next item of `items`.
fn draw(n: u64, count: Numbers, unit: FloatWidth, items: &mut rustymath::random::Items) -> Value {
    let mut item = items.item();
    if n == 0 { Value::Number(Number::float(rustymath::random::unit(rand::Rng::next_u64(&mut item)), unit)) } else { Value::Number(count.number(rustymath::random::below(n, &mut item))) }
}

/// Whether integers of `count` wrap, as fixed integers do, and exact ones under static kinds: a bound of 0, which rolls a float, can't give one of them.
pub(crate) fn wraps(count: Numbers, numeric: Numeric) -> bool { matches!(count, Numbers::Fixed(_)) || count == Numbers::Exact && numeric.is_static() }

/// DOMAIN for a bound of 0 where the roll gives integers that wrap (`wraps`). A bound of 0 rolls a float, which their result can't hold.
fn zero_bound(count: Numbers) -> Result<(), Error> {
    if wraps(count, Numeric::current()) { Err(Error::domain("a bound of 0 rolls a float, which an integer result can't hold")) } else { Ok(()) }
}

fn roll(right: &Value, span: &Span, fill: bool, items: &mut rustymath::random::Items) -> Result<Value, Error> {
    let fill = fill || right.is_empty();
    let item = |e: Value, items: &mut rustymath::random::Items| {
        if let a @ Value::Array(_) = e { return roll(&a, span, fill, items); }
        let count = counted(&e);
        if fill { return Ok(Value::Number(count.number(0))); }
        let unit = if let Numbers::Float(width) = count { width } else { Numeric::current().width };
        let n = numeric(&e)?.nonnegative_integer::<u64>().described("roll needs a nonnegative whole number")?;
        if n == 0 { zero_bound(count)?; }
        Ok(draw(n, count, unit, items))
    };
    if right.is_atom() { return item(right.clone(), items); }
    let prototype = if right.is_empty() { item(right.prototype(), items)? } else { Value::integer(0) };
    let count = match right.as_items() {
        Items::Compact(View::Integers(_)) => Numbers::Exact, Items::Fixed(ints) => Numbers::Fixed(ints.tag()),
        Items::Compact(View::Floats(f)) => Numbers::Float(f.tag()), _ => Numbers::Float(Numeric::current().width)
    };
    let unit = if let Numbers::Float(width) = count { width } else { Numeric::current().width };
    let mut data = Gather::items(right.len());
    match right.as_items() {
        nested if nested.nested() => { for e in nested.values() { data.add(item(e.into_owned(), items)?); } }
        bounds if !fill => {
            let bounds = bounds.nonnegative_integers::<u64>().described("roll needs a nonnegative whole number")?;
            // A bound of 0 draws a float between 0 and 1, unless integers wrap (`zero_bound`). The other draws keep their exactness beside it.
            if !bounds.contains(&0) {
                let mut draws = Vec::with_capacity(bounds.len());
                rustymath::random::roll(rustymath::Arg::Many(&bounds), bounds.len(), items, &mut draws);
                return generated_items(right.shape().to_vec(), draws.into_iter(), bounds.iter().copied().max().unwrap_or(1), count)
                    .and_then(|v| v.with_layout(right.layout().clone())).described("invalid roll result");
            }
            zero_bound(count)?;
            if bounds.iter().all(|&n| n == 0) {
                let mut draws = Vec::with_capacity(bounds.len());
                rustymath::random::units(bounds.len(), items, &mut draws);
                return right.layout().floats(unit, draws).described("invalid roll result");
            }
            for n in bounds { data.add(draw(n, count, unit, items)); }
        }
        _ => data.fill(&Value::Number(count.number(0)), right.len()),
    }
    data.finish(right.layout().clone(), prototype).described("invalid roll result")
}

pub(crate) fn deal(left: &Value, right: &Value, stream: rustymath::random::Stream) -> Result<Value, Error> {
    let parsed = |a: &Value| {
        if a.shape().len() > 1 { return Err(Error::new(Kind::Rank, "deal needs units or singleton vectors")); }
        if !a.is_singleton() { return Err(Error::new(Kind::Length, "deal needs one count per argument")); }
        let item = a.at(0);
        let n = numeric(&item)?;
        Ok((n.nonnegative_integer().described("deal needs nonnegative whole counts")?, counted(&item)))
    };
    let ((n, n_count), (total, total_count)) = (parsed(left)?, parsed(right)?);
    if n > total { return Err(Error::domain("cannot deal more items than the population")); }
    generated_len(&[n])?;
    // The draws are below the total, so they take its type, unless either argument is a float.
    let count = match (n_count, total_count) {
        (Numbers::Float(a), Numbers::Float(b)) => Numbers::Float(a.max(b)), (Numbers::Float(width), _) | (_, Numbers::Float(width)) => Numbers::Float(width), (_, total) => total
    };
    let mut draws = Vec::new();
    rustymath::random::deal(n as u64, total as u64, stream, &mut draws);
    generated_items(vec![n], draws.into_iter(), total as u64, count).described("invalid deal result")
}

pub(crate) fn roles(call: &CallView<'_>) -> Result<Roles, Error> {
    let Operation::Primitive { primitive, .. } = call.operation else { return Ok(Roles(0)) };
    let roots: Vec<_> = call.explicit_argument_roots().collect();
    let positions: Vec<_> = roots.iter().map(|&root| call.root_boundary(root)).collect();
    let count = positions.len();
    if count == 0 || count > 2 { return Ok(Roles(0)); }
    let dyadic = count == 2;
    let circle = dyadic && matches!(primitive, Primitive::Math(Math::Circle | Math::Arc) | Primitive::RealCircle { .. });
    let mut roles = if circle { Roles(0).with(positions[0], ReadKind::Value) } else { Roles(0) };
    if !call.mode.dynamic { return Ok(roles); }
    let start = usize::from(circle);
    let mut types = [DType::Boolean; 2];
    let mut ranges = [rustymath::facts::Range::ANY; 2];
    for (to, &position) in positions[start..].iter().enumerate() {
        let Some(desc) = call.desc(position) else { return Ok(roles) };
        if matches!(desc.kind.dtype(), DType::Complex(_) | DType::Character) { return Ok(roles); }
        types[to] = desc.kind.dtype(); ranges[to] = desc.range.unwrap_or(rustymath::facts::Range::ANY);
    }
    let integer_division = dyadic && matches!(primitive, Primitive::Arithmetic(Arithmetic::Divide))
        && types[..count].iter().all(|kind| kind.is_integral());
    let code = if circle {
        call.input(positions[0]).and_then(|input| match input.value { Value::Number(number) => number.as_integer(), _ => None })
            .or_else(|| call.desc(positions[0])?.range?.constant().filter(|x| x.fract() == 0.0).map(|x| x as i64))
    } else { None };
    let variant = rustymath::facts::Variant { dtype: DType::Float(FloatWidth::F64), checked: true, wrapping: false, mathish: call.mode.mathish };
    let has_real_kind = match object_kernel(*primitive, dyadic, Numeric::EXACT, code, variant, &types[..count - start]) {
        Ok(Some(kernel)) => matches!(kernel.result_dtype(), DType::Complex(_)) || kernel.domain(&ranges[..count - start]),
        _ => !circle,
    };
    if integer_division || !has_real_kind { for &position in &positions[start..] { roles = roles.with(position, ReadKind::Kind); } }
    Ok(roles)
}

pub(crate) fn kernel(call: &CallView<'_>, variant: rustymath::facts::Variant) -> Result<Option<rustymath::operation::Kernel>, Error> {
    let (primitive, dyadic) = match call.operation {
        Operation::Primitive { primitive, .. } => (*primitive, call.explicit_arguments().len() == 2),
        Operation::Fold { primitive, .. } => (*primitive, true),
        Operation::At { combine: Some(primitive), .. } => (*primitive, true),
        _ => return Ok(None),
    };
    call.kernel_for(primitive, dyadic, variant)
}
fn resolved<K: rustymath::facts::Formula<N>, const N: usize>(object: K, variant: rustymath::facts::Variant,
    types: &[rustymath::buffer::DType]) -> Result<rustymath::operation::Kernel, Error> {
    let types = if let [kind] = types { [*kind; N] }
        else { types.try_into().map_err(|_| Error::domain("numeric operand count changed"))? };
    rustymath::facts::Formula::resolve(&object, variant, types).map_err(Into::into)
}

pub(crate) fn object_kernel(primitive: Primitive, dyadic: bool, numeric: Numeric, code: Option<i64>,
    variant: rustymath::facts::Variant, types: &[rustymath::buffer::DType]) -> Result<Option<rustymath::operation::Kernel>, Error> {
    use crate::number::{Arithmetic::*, Math::*};
    use Primitive::{Arithmetic as A, Math as M, Special as S};
    use rustymath::{self as rm, tolerant as tol};
    macro_rules! one { ($object:expr) => { resolved::<_, 1>($object, variant, types)? }; }
    macro_rules! two { ($object:expr) => { resolved::<_, 2>($object, variant, types)? }; }
    let tolerance = numeric.tolerance_at(match variant.dtype { DType::Float(width) => width, DType::Complex(width) => width.into(), _ => numeric.width });
    Ok(Some(match (primitive, dyadic) {
        (A(Plus), false) => one!(rm::Conj), (A(Plus), true) => two!(rm::Add),
        (A(Minus), false) => one!(rm::Negate), (A(Minus), true) => two!(rm::Subtract),
        (A(Times), false) => one!(rm::Sign), (A(Times), true) => two!(rm::Multiply),
        (A(Divide), false) => one!(rm::Reciprocal),
        (A(Divide), true) if variant.mathish => two!(rm::MathishDivide), (A(Divide), true) => two!(rm::Divide),
        (M(Magnitude), false) => one!(rm::Abs), (M(Magnitude), true) => two!(tol::Residue(tolerance)),
        (M(Floor), false) => one!(tol::Floor(tolerance)), (M(Floor), true) => two!(rm::Minimum),
        (M(Ceiling), false) => one!(tol::Ceil(tolerance)), (M(Ceiling), true) => two!(rm::Maximum),
        (M(Power), false) => one!(rm::Exp), (M(Power), true) => two!(rm::Pow),
        (M(Log), false) => one!(rm::Ln),
        (M(Log), true) if variant.mathish => two!(rm::MathishLogb), (M(Log), true) => two!(rm::Logb),
        (M(Pi), false) => one!(rm::PiTimes), (M(Pi), true) => two!(rm::PiTimes),
        (M(Root), false) => one!(rm::Sqrt), (M(Root), true) => two!(rm::Root),
        (M(Factorial), false) => one!(rm::Factorial), (M(Factorial), true) => two!(rm::Flipped(rm::Binom)),
        (M(Gcd), true) => two!(tol::Gcd(tolerance)), (M(Lcm), true) => two!(tol::Lcm(tolerance)),
        (M(Not), false) => one!(rm::Not), (M(Nand), true) => two!(rm::Nand), (M(Nor), true) => two!(rm::Nor),
        (M(Circle), false) => one!(rm::Cis),
        (M(op @ (Circle | Arc)), true) => {
            let code = code.ok_or_else(|| Error::domain("circle code must be a scalar integer"))?;
            let kind = *types.last().ok_or_else(|| Error::domain("circle needs an argument"))?;
            return rm::operation::Kernel::circle(code, op == Arc, false, variant, [kind]).map_err(Into::into);
        }
        (Primitive::RealCircle { inverse }, true) => {
            let code = code.ok_or_else(|| Error::domain("circle code must be a scalar integer"))?;
            let kind = *types.last().ok_or_else(|| Error::domain("circle needs an argument"))?;
            return rm::operation::Kernel::circle(code, inverse, true, variant, [kind]).map_err(Into::into);
        }
        (Primitive::Compare(comparison), true) => match comparison {
            Comparison::Equal => two!(tol::Equal(tolerance)), Comparison::NotEqual => two!(tol::NotEqual(tolerance)),
            Comparison::Less => two!(tol::Less(tolerance)), Comparison::LessEqual => two!(tol::LessEqual(tolerance)),
            Comparison::Greater => two!(tol::Greater(tolerance)), Comparison::GreaterEqual => two!(tol::GreaterEqual(tolerance)),
        },
        (Primitive::Bitwise(table), false) => one!(rm::Bitwise(table)), (Primitive::Bitwise(table), true) => two!(rm::Bitwise(table)),
        (Primitive::RealSqrt, false) => one!(rm::RealSqrt), (Primitive::RealLn, false) => one!(rm::RealLn),
        (Primitive::RealPow, true) => two!(rm::RealPow),
        (Primitive::Log1p, false) => one!(rm::Log1p), (Primitive::Expm1, false) => one!(rm::Expm1),
        (S(Special::Sign), false) => one!(rm::Sign), (S(Special::Floor), false) => one!(tol::Floor(tol::Tolerance::EXACT)),
        (S(Special::LambertW), false) => one!(rm::LambertW),
        (S(Special::Real), false) => one!(rm::Real), (S(Special::Imag), false) => one!(rm::Imag), (S(Special::Angle), false) => one!(rm::Angle),
        (S(Special::Gammaln), false) => one!(rm::Kept(rm::Lgamma)), (S(Special::Digamma), false) => one!(rm::Kept(rm::Digamma)),
        (S(Special::Erf), false) => one!(rm::Kept(rm::Erf)), (S(Special::Erfc), false) => one!(rm::Kept(rm::Erfc)),
        (S(Special::Erfinv), false) => one!(rm::Kept(rm::Erfinv)),
        (S(Special::Beta), true) => two!(rm::Kept(rm::Beta)), (S(Special::Gammainc), true) => two!(rm::Kept(rm::Gammainc)),
        (S(Special::Polygamma), true) => two!(rm::Kept(rm::Polygamma)),
        (S(Special::BinomDn), true) => two!(rm::Kept(rm::BinomDn)), (S(Special::BinomDk), true) => two!(rm::Kept(rm::BinomDk)),
        (Primitive::Exact(Exact::Floor), false) => one!(tol::Floor(tolerance)),
        (Primitive::Exact(Exact::Ceiling), false) => one!(tol::Ceil(tolerance)),
        (Primitive::Exact(Exact::Quotient), true) if variant.mathish => two!(tol::MathishFloorDivide(tolerance)),
        (Primitive::Exact(Exact::Quotient), true) => two!(tol::FloorDivide(tolerance)),
        (Primitive::Exact(Exact::Ratio), true) => two!(rm::Divide),
        _ => return Ok(None),
    }))
}

fn map_arguments(call: &CallView<'_>) -> Result<(Option<usize>, usize), Error> {
    let mut positions = call.data_positions();
    let first = positions.next().ok_or_else(|| Error::domain("a numeric call needs an argument"))?;
    let pair = match positions.next() { None => (None, first), Some(second) => (Some(first), second) };
    if positions.next().is_some() { return Err(Error::domain("too many numeric arguments")); }
    Ok(pair)
}

fn map_variant(call: &CallView<'_>) -> Result<rustymath::facts::Variant, Error> {
    call.variant(call.numeric()?.kind(call.operand_kind()?.map_or(DType::Character, crate::number::Kind::dtype)))
}

fn map_geometry(call: &CallView<'_>) -> Result<(Layout, bool, Option<Mapping>, Mapping), Error> {
    let (left, right) = map_arguments(call)?;
    let right = call.desc(right).ok_or_else(Error::planning)?;
    let Some(left) = left else { return Ok((right.layout.clone(), right.atom, None, Mapping::Linear(1))); };
    let left = call.desc(left).ok_or_else(Error::planning)?;
    let pairing = match call.operation { Operation::Primitive { pairing, .. } => *pairing, _ => Pairing::Axes };
    let agreement = match pairing {
        Pairing::Explicit => {
            let axis = call.value(call.explicit - 1, ReadKind::Structure)?;
            let axes = axes(&resolve_axes(&axis, &right.layout)?, right.shape().len())?;
            Agreement::with_axes(&left.layout, &right.layout, &axes)?
        }
        Pairing::Axes => Agreement::new(&left.layout, &right.layout)?,
        Pairing::Matrices { .. } => return Err(Error::domain("elementwise calls do not use matrix pairing")),
    };
    Ok((agreement.layout, left.atom && right.atom, Some(agreement.left), agreement.right))
}

pub(super) fn describe_map(call: &CallView<'_>) -> Result<Option<crate::program::Description>, Error> {
    let (left, right) = map_arguments(call)?;
    if call.desc(right).is_none() || left.is_some_and(|left| call.desc(left).is_none()) { return Ok(None); }
    if let Operation::Recipe(operation) = call.operation && let Some(conversion) = operation.conversion(call.desc(right).ok_or_else(Error::planning)?.dtype()) {
        let source = call.desc(right).ok_or_else(Error::planning)?;
        if source.segments.is_some() { return Ok(None); }
        let kind = crate::number::Kind::stored(conversion.output, conversion.output.is_integral(), false);
        return Ok(Some(Outputs::one(Desc { kind, atom: source.atom, layout: source.layout.clone(), range: None, segments: None })));
    }
    let Some(kernel) = call.kernel(map_variant(call)?)? else { return Ok(None) };
    let (layout, atom, _, _) = map_geometry(call)?;
    let mut ranges = [rustymath::facts::Range::ANY; 2];
    let mut count = 0;
    for position in left.into_iter().chain([right]) {
        let desc = call.desc(position).expect("numeric arguments have descriptions");
        if desc.segments.is_some() { return Ok(None); }
        ranges[count] = desc.range.unwrap_or(rustymath::facts::Range::ANY); count += 1;
    }
    let numeric = call.numeric()?;
    if call.mode.dynamic && !kernel.domain(&ranges[..count]) { return Ok(None); }
    let dtype = kernel.result_dtype();
    if !numeric.wrapping() && kernel.has_exact() && !dtype.is_integral()
        && left.into_iter().chain([right]).all(|position| call.desc(position).is_some_and(|desc| desc.kind.is_exact())) { return Ok(None); }
    let kind = crate::number::Kind::stored(dtype, numeric.fixed.is_some(), false);
    Ok(Some(Outputs::one(Desc { kind, atom, layout, range: Some(kernel.range(&ranges[..count])), segments: None })))
}

pub(super) fn plan_map(call: &CallView<'_>, _: &[Desc]) -> Result<Step, Error> {
    let (left, right) = map_arguments(call)?;
    if let Operation::Recipe(operation) = call.operation && let Some(conversion) = operation.conversion(call.desc(right).ok_or_else(Error::planning)?.dtype()) {
        return Ok(Step::Convert { arg: right as u32, conversion });
    }
    Ok(Step::Map { left: left.map(|position| position as u32), right: right as u32, variant: map_variant(call)? })
}

pub(super) fn map_maps(call: &CallView<'_>, _: &Step, _: &[Desc], set: &mut dyn FnMut(usize, Mapping))
    -> Result<Option<rustymath::agreement::ReadPlan>, Error> {
    let (left, right) = map_arguments(call)?;
    let (_, _, left_map, right_map) = map_geometry(call)?;
    if let Some(left) = left { set(left, left_map.expect("a pair has two maps")); }
    set(right, right_map);
    Ok(None)
}

pub(super) fn code_recipe(call: &CallView<'_>) -> Result<Outputs<Value>, Error> { code_map(call) }

pub(super) fn code(call: &CallView<'_>) -> Result<Outputs<Value>, Error> {
    use crate::number::{Arithmetic::*, Math::*};
    let Operation::Primitive { primitive, .. } = call.operation else { unreachable!("a primitive row") };
    let positions: Vec<_> = call.explicit_arguments().collect();
    let right = call.input(*positions.last().ok_or_else(|| Error::domain("a function needs an argument"))?).ok_or_else(Error::planning)?.value;
    let left = if positions.len() == 2 { Some(call.input(positions[0]).ok_or_else(Error::planning)?.value) } else { None };
    let apply = |primitive, args: &[Value], from: &[Option<usize>]| call.apply(Operation::Primitive { primitive, pairing: Pairing::Axes }, args, from).map(Outputs::one);
    match (primitive, left) {
        (Primitive::Math(Nand), None) => apply(Primitive::Arithmetic(Times), &[right.clone(), right.clone()], &[positions.last().copied(), positions.last().copied()]),
        (Primitive::Math(Nor), None) => apply(Primitive::Arithmetic(Plus), &[right.clone(), right.clone()], &[positions.last().copied(), positions.last().copied()]),
        (Primitive::Math(Gcd | Lcm), None) => complex_parts(call, *primitive == Primitive::Math(Lcm)),
        (Primitive::Compare(Comparison::LessEqual | Comparison::GreaterEqual), None) => {
            let op = if *primitive == Primitive::Compare(Comparison::LessEqual) { Minus } else { Plus };
            apply(Primitive::Arithmetic(op), &[right.clone(), Value::weak_integer(1)], &[positions.last().copied(), None])
        }
        (Primitive::Compare(_), None) | (Primitive::Math(Not), Some(_)) => super::host_primitive(call),
        _ => code_map(call),
    }
}

fn code_map(call: &CallView<'_>) -> Result<Outputs<Value>, Error> {
    let numeric = call.numeric()?;
    let (left, right) = map_arguments(call)?;
    let right = call.value(right, ReadKind::Value)?;
    let left = left.map(|position| call.value(position, ReadKind::Value)).transpose()?;
    let scalar = |left: Option<&Value>, right: &Value, prototype: bool| {
        let args: Vec<_> = left.into_iter().chain([right]).collect();
        let kinds: Vec<_> = args.iter().filter_map(|value| value.kind()).filter(|kind| kind.dtype() != DType::Character).collect();
        let numeric = numeric.for_kinds(kinds.iter().copied()).map_err(Error::domain)?;
        let kind = kinds.into_iter().reduce(|left, right| left.join(right).expect("joined numeric kinds"));
        let dtype = kind.map_or(DType::Float(numeric.width), |kind| numeric.kind(kind.dtype()));
        let variant = rustymath::facts::Variant { dtype, checked: call.mode.dynamic, wrapping: numeric.wrapping(), mathish: numeric.mathish_at(crate::array::float_width_of(dtype)) };
        let types: Vec<_> = args.iter().map(|value| value.dtype().unwrap_or(dtype)).collect();
        let resolve = |variant| match call.operation {
            Operation::Recipe(operation) => Ok(operation.kernel(variant)),
            Operation::Primitive { primitive, .. } => object_kernel(*primitive, call.explicit_arguments().len() == 2, numeric,
                call.parameter(0).and_then(Value::as_number).and_then(Number::as_integer), variant, &types),
            _ => unreachable!("a mapped operation"),
        };
        let mut kernel = resolve(variant)?.ok_or_else(|| Error::domain("numeric formula does not take these kinds"))?;
        let ranges: Vec<_> = args.iter().map(|value| value.range().unwrap_or(rustymath::facts::Range::ANY)).collect();
        if !prototype && call.mode.dynamic && !kernel.domain(&ranges) && !matches!(dtype, DType::Complex(_) | DType::Character) {
            if let Ok(Some(complex)) = resolve(rustymath::facts::Variant { dtype: DType::Float(numeric.width).complex(), ..variant }) { kernel = complex; }
        }
        if prototype { return Ok(Desc::new(crate::number::Kind::stored(kernel.result_dtype(), numeric.fixed.is_some(), false), true, Layout::default()).zero()); }
        crate::number::calculate(&kernel, &args, numeric)
    };
    Ok(Outputs::one(pervade(left.as_deref(), &right, numeric, &|left, right| scalar(left, right, false), &|left, right| scalar(left, right, true))?))
}

pub(crate) fn product_kernels(call: &CallView<'_>) -> Result<(rustymath::operation::Kernel, rustymath::operation::Kernel), Error> {
    let Operation::Product { map, fold, .. } = call.operation else { return Err(Error::planning()); };
    let numeric = call.numeric()?;
    let map = call.kernel_for(*map, true, map_variant(call)?)?.ok_or_else(|| Error::domain("product mapping has no numerical body"))?;
    let dtype = map.result_dtype();
    let fold = object_kernel(*fold, true, numeric, None, call.variant(dtype)?, &[dtype; 2])?
        .ok_or_else(|| Error::domain("product reduction has no numerical body"))?;
    Ok((map, fold))
}

pub(crate) fn describe_fold(call: &CallView<'_>) -> Result<Option<crate::program::Description>, Error> {
    let geometry = crate::plan::fold(call)?;
    let source = call.desc(geometry.arg).ok_or_else(Error::planning)?;
    if source.segments.is_some() { return Ok(None); }
    let Some(kernel) = call.kernel(map_variant(call)?)? else { return Ok(None); };
    let items = source.range.unwrap_or(rustymath::facts::Range::ANY);
    let seed = geometry.seed.and_then(|seed| call.desc(seed)).and_then(|seed| seed.range);
    let length = geometry.dims()[1];
    let mut range = if length == 0 { rustymath::facts::Range::of(kernel.identity()) }
        else { rustymath::facts::Range::folded(items, seed, Some(length), |args| kernel.range(&args)) };
    if geometry.scan { range = range.join(items); }
    let dtype = kernel.total_dtype().unwrap_or(kernel.result_dtype());
    let numeric = call.numeric()?;
    let kind = crate::number::Kind::stored(dtype, numeric.fixed.is_some(), false);
    Ok(Some(Outputs::one(Desc { kind, atom: geometry.atom, layout: geometry.result, range: Some(range), segments: None })))
}

pub(crate) fn plan_fold(call: &CallView<'_>, _: &[Desc]) -> Result<Step, Error> {
    let geometry = crate::plan::fold(call)?;
    Ok(Step::Fold { arg: geometry.arg as u32, seed: geometry.seed.map(|seed| seed as u32), axis: geometry.axis, variant: map_variant(call)? })
}

pub(crate) fn maps_fold(call: &CallView<'_>, _: &Step, _: &[Desc], set: &mut dyn FnMut(usize, Mapping))
    -> Result<Option<rustymath::agreement::ReadPlan>, Error> {
    let geometry = crate::plan::fold(call)?;
    set(geometry.arg, Mapping::Linear(1));
    if let Some(seed) = geometry.seed {
        let desc = call.desc(seed).ok_or_else(Error::planning)?;
        let agreement = Agreement::new(&desc.layout, &geometry.result)?;
        if agreement.layout != geometry.result { return Err(Error::new(Kind::Length, "fold seed does not agree with its result cells")); }
        set(seed, agreement.left);
    }
    Ok(None)
}

pub(crate) fn describe_product(call: &CallView<'_>) -> Result<Option<crate::program::Description>, Error> {
    let geometry = crate::plan::product(call)?;
    let (map, fold) = product_kernels(call)?;
    let ranges = geometry.args.map(|position| call.desc(position).and_then(|desc| desc.range).unwrap_or(rustymath::facts::Range::ANY));
    let shape = geometry.plan.iteration.shape();
    let count = shape[shape.len() - geometry.plan.contracted..].iter().product();
    let range = rustymath::facts::Range::folded(map.range(&ranges), None, Some(count), |args| fold.range(&args));
    let dtype = fold.total_dtype().unwrap_or(fold.result_dtype());
    let numeric = call.numeric()?;
    let kind = crate::number::Kind::stored(dtype, numeric.fixed.is_some(), false);
    let atom = geometry.plan.layout.shape().is_empty();
    Ok(Some(Outputs::one(Desc { kind, atom, layout: geometry.plan.layout.into(), range: Some(range), segments: None })))
}

pub(crate) fn plan_product(call: &CallView<'_>, _: &[Desc]) -> Result<Step, Error> {
    let geometry = crate::plan::product(call)?;
    Ok(Step::Product { left: geometry.args[0] as u32, right: geometry.args[1] as u32 })
}

pub(crate) fn maps_product(call: &CallView<'_>, _: &Step, _: &[Desc], set: &mut dyn FnMut(usize, Mapping))
    -> Result<Option<rustymath::agreement::ReadPlan>, Error> {
    let geometry = crate::plan::product(call)?;
    for (position, map) in geometry.args.into_iter().zip(geometry.plan.mappings) { set(position, map); }
    Ok(None)
}
