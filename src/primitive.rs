use crate::{
    array::{generated_len, Axis, MAX_GENERATED_ELEMENTS},
    number::{Arithmetic, Math},
    Array, Element, Error, ErrorKind, Number, Span,
};
use rand::Rng;
use std::{cmp::Ordering, collections::HashMap};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Hybrid { pub scan: bool, pub first: bool, pub axis: Option<usize> }
#[derive(Clone, Copy, Debug)]
pub(crate) enum OperatorKind {
    Each,
    Commute,
    Compose,
    Rank,
    Over,
    Behind,
    Product,
    Outer,
    Key,
    Power,
}
impl Hybrid { pub(crate) fn primitive(self) -> Primitive { if self.scan { Primitive::Expand(self.first) } else { Primitive::Replicate(self.first) } } }

#[derive(Clone, Copy, Debug)]
pub(crate) enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Primitive {
    Arithmetic(Arithmetic),
    Math(Math),
    Random,
    Identity(bool),
    Compare(Comparison),
    Iota,
    Depth,
    Where,
    Member,
    Union,
    Intersection,
    Find,
    Grade(bool),
    Index,
    Shape,
    Tally,
    Ravel,
    Enclose,
    Nest,
    Disclose,
    Take,
    Drop,
    Replicate(bool),
    Expand(bool),
    CatenateFirst,
    Reverse(bool),
    Transpose,
}

fn numeric<'a>(e: &'a Element, span: &Span) -> Result<&'a Number, Error> {
    match e { Element::Number(n) => Ok(n), _ => Err(span.error(ErrorKind::Domain, "expected numeric elements")) }
}
fn float(n: f64) -> Element { Element::Number(Number::try_from(n).expect("finite generated number")) }
fn selected(a: &Array, i: usize) -> Element { a.at(if a.is_singleton() { 0 } else { i }) }

fn float_binary(x: &[f64], y: &[f64], f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    if x.len() == 1 { y.iter().map(|&b| f(x[0], b)).collect() } else if y.len() == 1 { x.iter().map(|&a| f(a, y[0])).collect() } else { x.iter().zip(y).map(|(&a, &b)| f(a, b)).collect() }
}

fn float_apply(op: Primitive, left: Option<&[f64]>, right: &[f64], shape: &[usize], span: &Span) -> Result<Array, Error> {
    use crate::number::float_equal;
    use Arithmetic::*;
    use Comparison::*;
    let data = match (op, left) {
        (Primitive::Arithmetic(op), Some(x)) => match op {
            Plus => float_binary(x, right, |a, b| a + b),
            Minus => float_binary(x, right, |a, b| a - b),
            Times => float_binary(x, right, |a, b| a * b),
            Divide => float_binary(x, right, |a, b| if a == 0.0 && b == 0.0 { 1.0 } else { a / b }),
        },
        (Primitive::Arithmetic(op), None) => match op {
            Plus => right.to_vec(),
            Minus => right.iter().map(|a| -a).collect(),
            Times => right.iter().map(|&a| if a == 0.0 { 0.0 } else { a.signum() }).collect(),
            Divide => right.iter().map(|a| 1.0 / a).collect(),
        },
        (Primitive::Compare(op), Some(x)) => {
            let boolean = |b| f64::from(u8::from(b));
            match op {
                Equal => float_binary(x, right, |a, b| boolean(float_equal(a, b))),
                NotEqual => float_binary(x, right, |a, b| boolean(!float_equal(a, b))),
                Less => float_binary(x, right, |a, b| boolean(a < b && !float_equal(a, b))),
                Greater => float_binary(x, right, |a, b| boolean(a > b && !float_equal(a, b))),
                LessEqual => float_binary(x, right, |a, b| boolean(a < b || float_equal(a, b))),
                GreaterEqual => float_binary(x, right, |a, b| boolean(a > b || float_equal(a, b))),
            }
        }
        _ => unreachable!(),
    };
    Array::floats(shape.to_vec(), data).map_err(|k| {
        let zero_divisor = matches!(op, Primitive::Arithmetic(Divide))
            && right.iter().enumerate().any(|(i, &b)| {
                b == 0.0 && left.is_none_or(|x| if right.len() == 1 { x.iter().any(|&a| a != 0.0) } else { x[if x.len() == 1 { 0 } else { i }] != 0.0 })
            });
        span.error(k, if zero_divisor { "division by zero" } else { "result is not finite" })
    })
}

/// Singleton extension is shared selection, not a universal broadcasting policy.
/// Scalar functions preserve shape; replicate below agrees along its selected axis.
pub(crate) fn scalar_agreement<'a>(left: Option<&'a Array>, right: &'a Array, span: &Span) -> Result<&'a Array, Error> {
    let Some(left) = left else { return Ok(right); };
    if left.is_singleton() && right.is_singleton() { return Ok(if left.shape().len() > right.shape().len() { left } else { right }); }
    if left.is_singleton() { return Ok(right); }
    if right.is_singleton() { return Ok(left); }
    if left.shape() != right.shape() {
        let kind = if left.shape().len() == right.shape().len() { ErrorKind::Length } else { ErrorKind::Rank };
        return Err(span.error(kind, "array shapes do not agree"));
    }
    Ok(right)
}

impl Primitive {
    pub(crate) fn from_glyph(c: char) -> Option<Self> {
        use Arithmetic::*;
        use Comparison::*;
        Some(match c {
            '?' => Self::Random,
            '⊣' => Self::Identity(true),
            '⊢' => Self::Identity(false),
            '|' => Self::Math(Math::Magnitude),
            '⌊' => Self::Math(Math::Floor),
            '⌈' => Self::Math(Math::Ceiling),
            '*' => Self::Math(Math::Power),
            '⍟' => Self::Math(Math::Log),
            '○' => Self::Math(Math::Circle),
            '!' => Self::Math(Math::Factorial),
            '∨' => Self::Math(Math::Gcd),
            '∧' => Self::Math(Math::Lcm),
            '⍲' => Self::Math(Math::Nand),
            '⍱' => Self::Math(Math::Nor),
            '~' => Self::Math(Math::Not),
            '+' => Self::Arithmetic(Plus),
            '-' => Self::Arithmetic(Minus),
            '×' => Self::Arithmetic(Times),
            '÷' => Self::Arithmetic(Divide),
            '=' => Self::Compare(Equal),
            '≠' => Self::Compare(NotEqual),
            '<' => Self::Compare(Less),
            '≤' => Self::Compare(LessEqual),
            '>' => Self::Compare(Greater),
            '≥' => Self::Compare(GreaterEqual),
            '⍳' => Self::Iota,
            '≡' => Self::Depth,
            '⍸' => Self::Where,
            '∊' => Self::Member,
            '∪' => Self::Union,
            '∩' => Self::Intersection,
            '⍷' => Self::Find,
            '⍋' => Self::Grade(false),
            '⍒' => Self::Grade(true),
            '⌷' => Self::Index,
            '⍴' => Self::Shape,
            '≢' => Self::Tally,
            ',' => Self::Ravel,
            '⊂' => Self::Enclose,
            '⊆' => Self::Nest,
            '⊃' => Self::Disclose,
            '↑' => Self::Take,
            '↓' => Self::Drop,
            '⍪' => Self::CatenateFirst,
            '⌽' => Self::Reverse(false),
            '⊖' => Self::Reverse(true),
            '⍉' => Self::Transpose,
            _ => return None,
        })
    }
    pub(crate) fn call(self, left: Option<&Array>, right: &Array, span: &Span) -> Result<Array, Error> { self.call_axis(left, right, None, span) }
    pub(crate) fn call_axis(self, left: Option<&Array>, right: &Array, axis: Option<usize>, span: &Span) -> Result<Array, Error> {
        let construct = |shape, data| Array::from_parts(shape, data, float(0.0)).map_err(|k| span.error(k, "invalid array result"));
        if axis.is_some()
            && !matches!(
                self,
                Self::Replicate(_)
                    | Self::Expand(_)
                    | Self::Reverse(_)
                    | Self::Ravel
                    | Self::CatenateFirst
                    | Self::Drop
                    | Self::Take
                    | Self::Index
                    | Self::Enclose
                    | Self::Nest
            )
        { return Err(span.error(ErrorKind::Unsupported, "axis is not supported by this primitive")); }
        match self {
            Self::Identity(first) => return Ok(if first { left.unwrap_or(right) } else { right }.clone()),
            Self::Member => {
                return match left { Some(x) => membership(x, right, span), None => enlist(right, span) }
            }
            Self::Union => {
                return match left { Some(x) => union(x, right, span), None => unique(right, span) }
            }
            Self::Intersection => return intersection(left.ok_or_else(|| span.error(ErrorKind::Syntax, "intersection needs a left argument"))?, right, span),
            Self::Find => return find(left.ok_or_else(|| span.error(ErrorKind::Syntax, "find needs a left argument"))?, right, span),
            Self::Grade(down) => return grade(left, right, down, span),
            Self::Index => {
                return match left { Some(x) => squad(x, right, axis, span), None => Ok(right.clone()) }
            }
            Self::Math(Math::Not) if left.is_some() => return without(left.unwrap(), right, span),
            Self::Depth => {
                return match left {
                    Some(x) => construct(vec![], vec![float(u8::from(array_match(x, right, span)?) as f64)]),
                    None => construct(vec![], vec![float(depth(right) as f64)]),
                }
            }
            Self::Tally if left.is_some() => return construct(vec![], vec![float(u8::from(!array_match(left.unwrap(), right, span)?) as f64)]),
            Self::Compare(Comparison::NotEqual) if left.is_none() => return unique_mask(right, span),
            Self::Iota => {
                return match left { Some(x) => index_of(x, right, span), None => iota(right, span) }
            }
            Self::Where => {
                return match left { Some(x) => interval_index(x, right, span), None => where_indices(right, span) }
            }
            Self::Random if left.is_some() => return deal(left.unwrap(), right, span),
            Self::Replicate(first) | Self::Expand(first) => {
                return replicate(
                    left.ok_or_else(|| span.error(ErrorKind::Syntax, "replicate/expand needs a left argument"))?,
                    right,
                    first,
                    axis,
                    matches!(self, Self::Expand(_)),
                    span,
                )
            }
            Self::Reverse(first) => return rotate(left, right, axis.unwrap_or(if first { 0 } else { right.shape().len().saturating_sub(1) }), span),
            Self::Transpose => return transpose(left, right, span),
            Self::Ravel | Self::CatenateFirst if left.is_some() => return catenate(left.unwrap(), right, axis, matches!(self, Self::CatenateFirst), span),
            Self::CatenateFirst => {
                if axis.is_some() { return Err(span.error(ErrorKind::Unsupported, "table with axis is not implemented")); }
                let rows = right.shape().first().copied().unwrap_or(1);
                let columns = crate::array::element_count(right.shape().get(1..).unwrap_or(&[])).map_err(|k| span.error(k, "invalid table shape"))?;
                return right.with_shape(vec![rows, columns]).map_err(|k| span.error(k, "invalid table"));
            }
            Self::Shape if left.is_some() => return reshape(left.unwrap(), right, span),
            Self::Disclose => {
                return match left { Some(x) => pick(x, right, span), None => Ok(right.disclose()) }
            }
            Self::Enclose | Self::Nest => {
                if let Some(x) = left { return partition(x, right, axis, matches!(self, Self::Nest), span); }
                if axis.is_some() { return Err(span.error(ErrorKind::Unsupported, "monadic enclosure with axis is not implemented")); }
                if matches!(self, Self::Nest) && right.elements().chain(std::iter::once(right.prototype().clone())).any(|e| matches!(e, Element::Nested(_))) {
                    return Ok(right.clone());
                }
                return Array::new(vec![], vec![Element::Nested(right.clone())]).map_err(|k| span.error(k, "invalid enclosure"));
            }
            Self::Take | Self::Drop => {
                if left.is_none() && matches!(self, Self::Take) {
                    if axis.is_some() { return Err(span.error(ErrorKind::Unsupported, "mix with axis is not implemented yet")); }
                    let cells: Vec<_> = right.elements().map(|e| e.as_array()).collect();
                    return Array::assemble(right.shape(), &cells, &right.prototype().as_array()).map_err(|k| span.error(k, "cannot assemble cells"));
                }
                if left.is_none() { return split(right, axis, span); }
                return take_drop(matches!(self, Self::Take), left.unwrap(), right, axis, span);
            }
            Self::Shape | Self::Tally | Self::Ravel => {
                if left.is_some() { return Err(span.error(ErrorKind::Unsupported, "this dyadic primitive is not implemented yet")); }
                return match self {
                    Self::Shape => construct(vec![right.shape().len()], right.shape().iter().map(|&n| float(n as f64)).collect()),
                    Self::Tally => Array::scalar(right.shape().first().copied().unwrap_or(1) as f64).map_err(|k| span.error(k, "invalid tally")),
                    Self::Ravel => {
                        if axis.is_some() { return Err(span.error(ErrorKind::Unsupported, "ravel with axes is not implemented yet")); }
                        right.with_shape(vec![right.len()]).map_err(|k| span.error(k, "invalid ravel"))
                    }
                    _ => unreachable!(),
                };
            }
            _ => (),
        }
        if matches!(self, Self::Compare(_)) && left.is_none() {
            return Err(span.error(ErrorKind::Unsupported, "this monadic primitive is not implemented yet"));
        }
        self.scalar_apply(left, right, span, false)
    }

    fn scalar_apply(self, left: Option<&Array>, right: &Array, span: &Span, fill: bool) -> Result<Array, Error> {
        let model = scalar_agreement(left, right, span)?;
        if model.is_empty() {
            let prototype = self.scalar_item(left.map(Array::prototype), right.prototype(), span, true)?;
            return Array::empty(model.shape().to_vec(), prototype).map_err(|k| span.error(k, "invalid empty result"));
        }
        if matches!(self, Self::Arithmetic(_) | Self::Compare(_)) && right.as_floats().is_some() && left.is_none_or(|a| a.as_floats().is_some()) {
            return float_apply(self, left.map(|a| a.as_floats().unwrap()), right.as_floats().unwrap(), model.shape(), span);
        }
        let data =
            (0..model.len()).map(|i| self.scalar_item(left.map(|a| selected(a, i)).as_ref(), &selected(right, i), span, fill)).collect::<Result<_, _>>()?;
        Array::new(model.shape().to_vec(), data).map_err(|k| span.error(k, "invalid scalar result"))
    }

    fn scalar_item(self, left: Option<&Element>, right: &Element, span: &Span, fill: bool) -> Result<Element, Error> {
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Element::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Element::Nested(_)) || matches!(left, Some(Element::Nested(_))) {
            return self.scalar_apply(left.map(|e| e.as_array()).as_ref(), &right.as_array(), span, fill).map(Element::Nested);
        }
        if fill {
            return Ok(match (self, left, right) {
                (Self::Math(Math::Circle | Math::Log), _, _) | (Self::Math(Math::Power), None, _) => float(0.0),
                (Self::Math(_), None, Element::Number(y)) => Element::Number(y.result_zero(None)),
                (Self::Math(_), Some(Element::Number(x)), Element::Number(y)) => Element::Number(y.result_zero(Some(x))),
                (Self::Arithmetic(_), None, Element::Number(y)) => Element::Number(y.result_zero(None)),
                (Self::Arithmetic(_), Some(Element::Number(x)), Element::Number(y)) => Element::Number(y.result_zero(Some(x))),
                _ => float(0.0),
            });
        }
        match self {
            Self::Random => {
                let n = numeric(right, span)?.nonnegative_integer().map_err(|k| span.error(k, "roll needs a nonnegative integer"))?;
                if n > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "roll bound exceeds exact floating-point integers")); }
                let mut rng = rand::rng();
                let value = if n == 0 { rng.sample(rand::distr::Open01) } else { rng.random_range(1..=n) as f64 };
                Ok(float(value))
            }
            Self::Math(op) => {
                let y = numeric(right, span)?;
                let n = match left { Some(x) => numeric(x, span)?.math_dyad(op, y), None => y.math_monad(op) };
                n.map(Element::Number).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            Self::Arithmetic(op) => {
                let y = numeric(right, span)?;
                let n = match left { Some(x) => numeric(x, span)?.dyad(op, y), None => y.monad(op) };
                n.map(Element::Number).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            Self::Compare(op) => {
                use Comparison::*;
                let x = left.unwrap();
                let result = match op {
                    Equal | NotEqual => {
                        let equal = match (x, right) {
                            (Element::Number(x), Element::Number(y)) => x.equal(y),
                            (Element::Character(x), Element::Character(y)) => Ok(x == y),
                            _ => Ok(false),
                        };
                        equal.map(|equal| if matches!(op, Equal) { equal } else { !equal })
                    }
                    _ => numeric(x, span)?.compare(numeric(right, span)?).map(|order| match op {
                        Less => order.is_lt(),
                        LessEqual => !order.is_gt(),
                        Greater => order.is_gt(),
                        GreaterEqual => !order.is_lt(),
                        _ => unreachable!(),
                    }),
                };
                result.map(|b| float(u8::from(b) as f64)).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            _ => unreachable!(),
        }
    }
}

pub(crate) fn array_match(left: &Array, right: &Array, span: &Span) -> Result<bool, Error> {
    if left.shape() != right.shape() { return Ok(false); }
    if left.is_empty() { return element_match(left.prototype(), right.prototype(), span); }
    for (x, y) in left.elements().zip(right.elements()) { if !element_match(&x, &y, span)? { return Ok(false); } }
    Ok(true)
}

fn element_match(left: &Element, right: &Element, span: &Span) -> Result<bool, Error> {
    match (left, right) {
        (Element::Number(x), Element::Number(y)) => x.equal(y).map_err(|m| span.error(ErrorKind::Domain, m)),
        (Element::Character(x), Element::Character(y)) => Ok(x == y),
        (Element::Nested(x), Element::Nested(y)) => array_match(x, y, span),
        _ => Ok(false),
    }
}

fn depth(right: &Array) -> isize {
    let items = right.elements().chain(right.is_empty().then(|| right.prototype().clone()));
    if items.clone().all(|e| !matches!(e, Element::Nested(_))) { return isize::from(!right.is_scalar()); }
    let ds: Vec<_> = items
        .map(|e| match e { Element::Nested(a) => depth(&a), _ => 0 })
        .collect();
    let n = 1 + ds.iter().map(|d| d.abs()).max().unwrap();
    if ds.iter().all(|&d| d >= 0 && d == ds[0]) { n } else { -n }
}

fn without(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "without needs a scalar or vector left argument")); }
    let mut data = Vec::new();
    for x in left.elements() { if !contains(right, &x, span)? { data.push(x); } }
    Array::from_parts(vec![data.len()], data, left.prototype().clone()).map_err(|k| span.error(k, "invalid without result"))
}

fn contains(array: &Array, element: &Element, span: &Span) -> Result<bool, Error> {
    for item in array.elements() { if element_match(&item, element, span)? { return Ok(true); } }
    Ok(false)
}

fn membership(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    let data = left.elements().map(|e| contains(right, &e, span).map(|b| float(u8::from(b) as f64))).collect::<Result<_, _>>()?;
    Array::from_parts(left.shape().to_vec(), data, float(0.0)).map_err(|k| span.error(k, "invalid membership result"))
}

fn enlist(right: &Array, span: &Span) -> Result<Array, Error> {
    fn append(array: &Array, data: &mut Vec<Element>, span: &Span) -> Result<(), Error> {
        for item in array.elements() {
            if let Element::Nested(a) = item { append(&a, data, span)?; } else {
                if data.len() == MAX_GENERATED_ELEMENTS { return Err(span.error(ErrorKind::Limit, "enlist exceeds array limits")); }
                data.push(item);
            }
        }
        Ok(())
    }
    let mut data = Vec::new();
    append(right, &mut data, span)?;
    let mut prototype = right.prototype();
    while let Element::Nested(a) = prototype { prototype = a.prototype(); }
    Array::from_parts(vec![data.len()], data, prototype.clone()).map_err(|k| span.error(k, "invalid enlist result"))
}

fn unique(right: &Array, span: &Span) -> Result<Array, Error> { replicate(&unique_mask(right, span)?, right, true, None, false, span) }

fn union(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    if left.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "union needs scalars or vectors")); }
    catenate(left, &without(right, left, span)?, None, false, span)
}

fn intersection(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    if left.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "intersection needs scalars or vectors")); }
    replicate(&membership(left, right, span)?, left, true, None, false, span)
}

fn find(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    let mut pattern_shape = vec![1; right.shape().len().saturating_sub(left.shape().len())];
    pattern_shape.extend_from_slice(left.shape());
    let mut data = vec![float(0.0); right.len()];
    if pattern_shape.len() <= right.shape().len() {
        for (flat, result) in data.iter_mut().enumerate() {
            let mut coords = vec![0; right.shape().len()];
            let mut n = flat;
            for axis in (0..coords.len()).rev() {
                coords[axis] = n % right.shape()[axis];
                n /= right.shape()[axis];
            }
            if coords.iter().zip(&pattern_shape).zip(right.shape()).any(|((&i, &len), &size)| len > size - i) { continue; }
            let mut matched = true;
            for (mut i, item) in left.elements().enumerate() {
                let (mut offset, mut stride) = (0, 1);
                for axis in (0..pattern_shape.len()).rev() {
                    offset += (coords[axis] + i % pattern_shape[axis]) * stride;
                    i /= pattern_shape[axis];
                    stride *= right.shape()[axis];
                }
                if !element_match(&item, &right.at(offset), span)? {
                    matched = false;
                    break;
                }
            }
            *result = float(u8::from(matched) as f64);
        }
    }
    Array::from_parts(right.shape().to_vec(), data, float(0.0)).map_err(|k| span.error(k, "invalid find result"))
}

fn unique_mask(right: &Array, span: &Span) -> Result<Array, Error> {
    let cells = right.cells(right.shape().len().saturating_sub(1)).map_err(|k| span.error(k, "invalid major cells"))?;
    let mut data = Vec::with_capacity(cells.len());
    let mut representatives = Vec::new();
    for cell in &cells {
        let mut unique = true;
        for earlier in &representatives {
            if array_match(earlier, cell, span)? {
                unique = false;
                break;
            }
        }
        if unique { representatives.push(cell.clone()); }
        data.push(float(u8::from(unique) as f64));
    }
    Array::from_parts(vec![data.len()], data, float(0.0)).map_err(|k| span.error(k, "invalid unique mask"))
}

fn coordinates(shape: &[usize], mut flat: usize) -> Array {
    let mut data = vec![float(0.0); shape.len()];
    for axis in (0..shape.len()).rev() {
        data[axis] = float((flat % shape[axis] + 1) as f64);
        flat /= shape[axis];
    }
    Array::from_parts(vec![data.len()], data, float(0.0)).unwrap()
}

fn iota(right: &Array, span: &Span) -> Result<Array, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "iota needs a scalar or vector shape")); }
    let shape = right
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid iota dimension")))
        .collect::<Result<Vec<_>, _>>()?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "iota exceeds array limits"))?;
    if right.is_singleton() { return Array::floats(shape, (1..=len).map(|i| i as f64).collect()).map_err(|k| span.error(k, "invalid iota")); }
    let prototype = Element::Nested(Array::from_parts(vec![shape.len()], vec![float(0.0); shape.len()], float(0.0)).unwrap());
    let data = (0..len).map(|i| Element::Nested(coordinates(&shape, i))).collect();
    Array::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid coordinate array"))
}

fn where_indices(right: &Array, span: &Span) -> Result<Array, Error> {
    let mut data = Vec::new();
    for (i, e) in right.elements().enumerate() {
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "where needs nonnegative integer counts"))?;
        if n > MAX_GENERATED_ELEMENTS - data.len() { return Err(span.error(ErrorKind::Limit, "where exceeds array limits")); }
        let index = if right.shape().len() == 1 { float((i + 1) as f64) } else { Element::Nested(coordinates(right.shape(), i)) };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype = if right.shape().len() == 1 { float(0.0) } else { Element::Nested(Array::from_parts(vec![right.shape().len()], vec![float(0.0); right.shape().len()], float(0.0)).unwrap()) };
    Array::from_parts(vec![data.len()], data, prototype).map_err(|k| span.error(k, "invalid where result"))
}

struct SearchCells { left: Vec<Array>, right: Vec<Array>, shape: Vec<usize> }

fn search_cells(left: &Array, right: &Array, span: &Span) -> Result<SearchCells, Error> {
    if left.is_scalar() { return Err(span.error(ErrorKind::Rank, "search needs a non-scalar left argument")); }
    let rank = left.shape().len() - 1;
    let split = right.shape().len().checked_sub(rank).ok_or_else(|| span.error(ErrorKind::Rank, "right argument has insufficient rank"))?;
    if left.shape()[1..] != right.shape()[split..] { return Err(span.error(ErrorKind::Length, "search cell shapes do not agree")); }
    Ok(SearchCells {
        left: left.cells(rank).map_err(|k| span.error(k, "invalid search cells"))?,
        right: right.cells(rank).map_err(|k| span.error(k, "invalid search cells"))?,
        shape: right.shape()[..split].to_vec(),
    })
}

fn index_of(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    let SearchCells { left: haystack, right: needles, shape } = search_cells(left, right, span)?;
    let mut data = Vec::with_capacity(needles.len());
    for y in &needles {
        let mut found = haystack.len();
        for (i, x) in haystack.iter().enumerate() {
            if array_match(x, y, span)? {
                found = i;
                break;
            }
        }
        data.push(float((found + 1) as f64));
    }
    Array::from_parts(shape, data, float(0.0)).map_err(|k| span.error(k, "invalid index-of result"))
}

fn grade_item(left: &Element, right: &Element) -> Ordering {
    match (left, right) {
        (Element::Number(x), Element::Number(y)) => x.grade_order(y),
        (Element::Character(x), Element::Character(y)) => x.cmp(y),
        (Element::Number(_), Element::Character(_)) => Ordering::Less,
        (Element::Character(_), Element::Number(_)) => Ordering::Greater,
        _ => grade_cell(&left.as_array(), &right.as_array()),
    }
}

fn grade_cell(left: &Array, right: &Array) -> Ordering {
    if left.is_empty() && right.is_empty() {
        return grade_item(left.prototype(), right.prototype()).then_with(|| left.shape().iter().rev().cmp(right.shape().iter().rev()));
    }
    if left.is_empty() || right.is_empty() { return left.len().cmp(&right.len()); }
    let rank_order = left.shape().len().cmp(&right.shape().len());
    let rank = left.shape().len().max(right.shape().len());
    let dimension = |a: &Array, axis: usize| { if axis < rank - a.shape().len() { 1 } else { a.shape()[axis - (rank - a.shape().len())] } };
    let mut count = left.len();
    let mut end_order = rank_order;
    let mut suffix = 1;
    for axis in (0..rank).rev() {
        let (x, y) = (dimension(left, axis), dimension(right, axis));
        if x != y {
            // The first padding item ends comparison; no padded array is allocated.
            count = x.min(y) * suffix;
            end_order = x.cmp(&y);
            break;
        }
        suffix *= x;
    }
    for (x, y) in left.elements().zip(right.elements()).take(count) {
        let order = grade_item(&x, &y);
        if !order.is_eq() { return order; }
    }
    end_order
}

fn grade(left: Option<&Array>, right: &Array, down: bool, span: &Span) -> Result<Array, Error> {
    if right.is_scalar() || left.is_some_and(Array::is_scalar) { return Err(span.error(ErrorKind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1]).map_err(|k| span.error(k, "grade result is too large"))?;
    let mut indices: Vec<usize> = (0..count).collect();
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Array| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Element::Character(_)) || a.elements().any(|e| !matches!(e, Element::Character(_))) {
                return Err(span.error(ErrorKind::Domain, "dyadic grade needs simple character arrays"));
            }
            Ok(a.elements()
                .map(|e| match e { Element::Character(c) => c, _ => unreachable!() })
                .collect())
        };
        let (alphabet, text) = (characters(collation)?, characters(right)?);
        let missing: Vec<_> = collation.shape().iter().rev().copied().collect();
        let mut weights: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, c) in alphabet.into_iter().enumerate() {
            let weight = weights.entry(c).or_insert_with(|| missing.clone());
            let mut offset = i;
            for (w, &size) in weight.iter_mut().zip(&missing) {
                *w = (*w).min(offset % size);
                offset /= size;
            }
        }
        let cells: Vec<_> = text.iter().map(|c| weights.get(c).unwrap_or(&missing)).collect();
        let size = crate::array::element_count(&right.shape()[1..]).map_err(|k| span.error(k, "invalid grade cell shape"))?;
        indices.sort_by(|&a, &b| {
            for (axis, _) in missing.iter().enumerate() {
                for j in 0..size {
                    let order = cells[a * size + j][axis].cmp(&cells[b * size + j][axis]);
                    if !order.is_eq() { return direction(order); }
                }
            }
            Ordering::Equal
        });
    }
    else {
        let cells = right.cells(right.shape().len() - 1).map_err(|k| span.error(k, "invalid grade cells"))?;
        indices.sort_by(|&a, &b| direction(grade_cell(&cells[a], &cells[b])));
    }
    Array::floats(vec![indices.len()], indices.into_iter().map(|i| (i + 1) as f64).collect()).map_err(|k| span.error(k, "invalid grade result"))
}

fn cell_order(left: &Array, right: &Array, span: &Span) -> Result<std::cmp::Ordering, Error> {
    for (x, y) in left.elements().zip(right.elements()) {
        let order = match (x, y) {
            (Element::Number(x), Element::Number(y)) => x.order(&y).map_err(|m| span.error(ErrorKind::Domain, m))?,
            (Element::Character(x), Element::Character(y)) => x.cmp(&y),
            (Element::Nested(x), Element::Nested(y)) => cell_order(&x, &y, span)?,
            _ => return Err(span.error(ErrorKind::Domain, "interval boundaries and values must have the same type")),
        };
        if !order.is_eq() { return Ok(order); }
    }
    Ok(left.len().cmp(&right.len()).then(left.shape().cmp(right.shape())))
}

fn interval_index(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    let SearchCells { left: boundaries, right: values, shape } = search_cells(left, right, span)?;
    for pair in boundaries.windows(2) {
        if cell_order(&pair[0], &pair[1], span)?.is_gt() { return Err(span.error(ErrorKind::Domain, "interval boundaries must be sorted")); }
    }
    let mut data = Vec::with_capacity(values.len());
    for value in &values {
        let (mut lo, mut hi) = (0, boundaries.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if cell_order(&boundaries[mid], value, span)?.is_gt() { hi = mid; } else { lo = mid + 1; }
        }
        data.push(float(lo as f64));
    }
    Array::from_parts(shape, data, float(0.0)).map_err(|k| span.error(k, "invalid interval index"))
}

fn deal(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    let count = |a: &Array| {
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "deal needs scalars or singleton vectors")); }
        if !a.is_singleton() { return Err(span.error(ErrorKind::Length, "deal needs one count per argument")); }
        numeric(&a.at(0), span)?.nonnegative_integer().map_err(|k| span.error(k, "deal needs nonnegative integer counts"))
    };
    let (n, total) = (count(left)?, count(right)?);
    if n > total { return Err(span.error(ErrorKind::Domain, "cannot deal more items than the population")); }
    generated_len(&[n]).map_err(|k| span.error(k, "deal exceeds array limits"))?;
    if total > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "deal population exceeds exact floating-point integers")); }
    let data = rand::seq::index::sample(&mut rand::rng(), total, n).into_iter().map(|i| float((i + 1) as f64)).collect();
    Array::from_parts(vec![n], data, float(0.0)).map_err(|k| span.error(k, "invalid deal result"))
}

fn reshape(dimensions: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    if dimensions.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "shape must be a scalar or vector")); }
    let shape = dimensions
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid dimension")))
        .collect::<Result<Vec<_>, _>>()?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "shape exceeds array limits"))?;
    if let Some(values) = right.as_floats() {
        let data = if values.is_empty() { vec![0.0; len] } else { values.iter().copied().cycle().take(len).collect() };
        return Array::floats(shape, data).map_err(|k| span.error(k, "invalid reshape"));
    }
    let data = if right.is_empty() { vec![right.prototype().clone(); len] } else { right.elements().cycle().take(len).collect() };
    Array::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid reshape"))
}

fn remap(right: &Array, shape: Vec<usize>, source: impl Fn(usize) -> Option<usize>, span: &Span) -> Result<Array, Error> {
    let len = generated_len(&shape).map_err(|k| span.error(k, "result exceeds array limits"))?;
    if let Some(values) = right.as_floats() {
        return Array::floats(shape, (0..len).map(|i| source(i).map_or(0.0, |j| values[j])).collect()).map_err(|k| span.error(k, "invalid structural result"));
    }
    let data = (0..len).map(|i| source(i).map_or_else(|| right.prototype().clone(), |j| right.at(j))).collect();
    Array::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid structural result"))
}

fn take_drop(take: bool, counts: &Array, right: &Array, axis: Option<usize>, span: &Span) -> Result<Array, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "take/drop counts must be a scalar or vector")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let old = if right.is_scalar() { vec![1; counts.len()] } else { right.shape().to_vec() };
    let mut shape = old.clone();
    let mut starts = vec![0i128; shape.len()];
    if counts.len() > shape.len() || (axis.is_some() && counts.len() != 1) { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    for (i, item) in counts.elements().enumerate() {
        let axis = axis.unwrap_or(i);
        if axis >= shape.len() { return Err(span.error(ErrorKind::Domain, "axis is outside array rank")); }
        let count = numeric(&item, span)?.integer().map_err(|k| span.error(k, "invalid take/drop count"))?;
        let n = count.unsigned_abs();
        shape[axis] = if take { n } else { old[axis].saturating_sub(n) };
        starts[axis] = if take && count < 0 { old[axis] as i128 - n as i128 } else if !take && count > 0 { n.min(old[axis]) as i128 } else { 0 };
    }
    remap(
        right,
        shape.clone(),
        |mut i| {
            let (mut source, mut stride) = (0, 1);
            for axis in (0..shape.len()).rev() {
                let coord = (i % shape[axis]) as i128 + starts[axis];
                i /= shape[axis];
                if coord < 0 || coord >= old[axis] as i128 { return None; }
                source += coord as usize * stride;
                stride *= old[axis];
            }
            Some(source)
        },
        span,
    )
}

fn replicate(counts: &Array, right: &Array, first: bool, axis: Option<usize>, expand: bool, span: &Span) -> Result<Array, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "replication counts must be a scalar or vector")); }
    let mut shape = if right.is_scalar() { vec![1] } else { right.shape().to_vec() };
    let axis = axis.unwrap_or(if first { 0 } else { shape.len() - 1 });
    let traversal = Axis::new(&shape, axis).map_err(|_| span.error(ErrorKind::Domain, "invalid replication axis"))?;
    if !expand && !counts.is_singleton() && traversal.len != 1 && counts.len() != traversal.len {
        return Err(span.error(ErrorKind::Length, "replication counts and data do not agree"));
    }
    let len = if expand || traversal.len == 1 { counts.len() } else { traversal.len };
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts: Vec<_> = counts
        .elements()
        .map(|e| numeric(&e, span)?.integer().map_err(|k| span.error(k, "replication count must be a representable integer")))
        .collect::<Result<_, _>>()?;
    shape[axis] = (0..len)
        .try_fold(0usize, |total, j| {
            let n = counts[if counts.len() == 1 { 0 } else { j }];
            total.checked_add(if expand && n == 0 { 1 } else { n.unsigned_abs() })
        })
        .ok_or_else(|| span.error(ErrorKind::Limit, "replication count overflow"))?;
    let size = generated_len(&shape).map_err(|k| span.error(k, "replication result exceeds array limits"))?;
    let mut data = Vec::with_capacity(size);
    if expand && traversal.len != 1 && counts.iter().filter(|&&n| n > 0).count() != traversal.len {
        return Err(span.error(ErrorKind::Length, "positive expansion counts must match the axis length"));
    }
    if size != 0 {
        for i in 0..traversal.outer {
            let mut consumed = 0;
            for j in 0..len {
                let n = counts[if counts.len() == 1 { 0 } else { j }];
                let repetitions = if expand && n == 0 { 1 } else { n.unsigned_abs() };
                if n <= 0 { data.extend(std::iter::repeat_n(right.prototype().clone(), repetitions * traversal.inner)); } else {
                    let index = if traversal.len == 1 { 0 } else if expand { consumed } else { j };
                    let start = traversal.offset(i, index, 0);
                    for _ in 0..repetitions { data.extend(right.items(start..start + traversal.inner)); }
                    consumed += 1;
                }
            }
        }
    }
    Array::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid replication result"))
}

fn rotate(counts: Option<&Array>, right: &Array, axis: usize, span: &Span) -> Result<Array, Error> {
    let shape = if right.is_scalar() { vec![1] } else { right.shape().to_vec() };
    let traversal = Axis::new(&shape, axis).map_err(|_| span.error(ErrorKind::Domain, "axis is outside array rank"))?;
    let mut frame = shape.clone();
    frame.remove(axis);
    let counts = counts
        .map(|a| {
            if !a.is_singleton() && a.shape() != frame { return Err(span.error(ErrorKind::Length, "rotation counts must match the axis frame")); }
            a.elements().map(|e| numeric(&e, span)?.integer().map_err(|k| span.error(k, "invalid rotation count"))).collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    remap(
        right,
        right.shape().to_vec(),
        |flat| {
            let k = flat % traversal.inner;
            let j = flat / traversal.inner % traversal.len;
            let i = flat / traversal.inner / traversal.len;
            let source = match &counts {
                None => traversal.len - 1 - j,
                Some(ns) => {
                    (j as isize + ns[if ns.len() == 1 { 0 } else { i * traversal.inner + k }].rem_euclid(traversal.len as isize)) as usize % traversal.len
                }
            };
            Some(traversal.offset(i, source, k))
        },
        span,
    )
}

fn catenate(left: &Array, right: &Array, axis: Option<usize>, first: bool, span: &Span) -> Result<Array, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(span.error(ErrorKind::Domain, "catenate axis is outside result rank")); }
    let promote = |a: &Array, other: &Array| -> Result<Vec<usize>, Error> {
        let mut shape = a.shape().to_vec();
        if a.is_scalar() {
            shape = if other.is_scalar() { vec![1] } else { other.shape().to_vec() };
            shape[axis] = 1;
        }
        else if shape.len() + 1 == rank { shape.insert(axis, 1); }
        else if shape.len() != rank { return Err(span.error(ErrorKind::Rank, "catenate ranks differ by more than one")); }
        Ok(shape)
    };
    let x = promote(left, right)?;
    let y = promote(right, left)?;
    if (0..rank).any(|i| i != axis && x[i] != y[i]) { return Err(span.error(ErrorKind::Length, "catenate frames differ")); }
    let mut shape = x.clone();
    shape[axis] = x[axis].checked_add(y[axis]).ok_or_else(|| span.error(ErrorKind::Limit, "catenate axis overflow"))?;
    let size = generated_len(&shape).map_err(|k| span.error(k, "catenate exceeds array limits"))?;
    let traversal = Axis::new(&shape, axis).map_err(|k| span.error(k, "invalid catenate axis"))?;
    let mut data = Vec::with_capacity(size);
    if size != 0 {
        for i in 0..traversal.outer {
            for (a, dims) in [(left, &x), (right, &y)] {
                let len = dims[axis] * traversal.inner;
                if a.is_scalar() { data.extend(std::iter::repeat_n(a.at(0), len)); } else { data.extend(a.items(i * len..(i + 1) * len)); }
            }
        }
    }
    Array::from_parts(shape, data, left.prototype().clone()).map_err(|k| span.error(k, "invalid catenate result"))
}

fn transpose(axes: Option<&Array>, right: &Array, span: &Span) -> Result<Array, Error> {
    let rank = right.shape().len();
    let axes = match axes {
        None => (0..rank).rev().collect::<Vec<_>>(),
        Some(a) => {
            if a.shape().len() > 1 || a.len() != rank { return Err(span.error(ErrorKind::Length, "transpose needs one axis per dimension")); }
            a.elements().map(|e| index(numeric(&e, span)?, rank, span)).collect::<Result<_, _>>()?
        }
    };
    let mut shape = vec![usize::MAX; axes.iter().max().map_or(0, |n| n + 1)];
    for (i, &axis) in axes.iter().enumerate() { shape[axis] = shape[axis].min(right.shape()[i]); }
    if shape.contains(&usize::MAX) { return Err(span.error(ErrorKind::Domain, "transpose axes must be consecutive from 1")); }
    remap(
        right,
        shape.clone(),
        |mut i| {
            let mut coords = vec![0; shape.len()];
            for axis in (0..shape.len()).rev() {
                coords[axis] = i % shape[axis];
                i /= shape[axis];
            }
            Some(axes.iter().zip(right.shape()).fold(0, |offset, (&axis, &size)| offset * size + coords[axis]))
        },
        span,
    )
}

fn split(right: &Array, axis: Option<usize>, span: &Span) -> Result<Array, Error> {
    if right.is_scalar() { return Array::new(vec![], vec![Element::Nested(right.clone())]).map_err(|k| span.error(k, "invalid split result")); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    let traversal = Axis::new(right.shape(), axis).map_err(|_| span.error(ErrorKind::Domain, "split axis is outside array rank"))?;
    let mut frame = right.shape().to_vec();
    frame.remove(axis);
    let size = generated_len(&frame).map_err(|k| span.error(k, "split exceeds array limits"))?;
    generated_len(&[traversal.len]).map_err(|k| span.error(k, "split cell exceeds array limits"))?;
    let prototype = Array::from_parts(vec![traversal.len], vec![right.prototype().clone(); traversal.len], right.prototype().clone()).unwrap();
    let mut data = Vec::with_capacity(size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for k in 0..traversal.inner {
            let cell = (0..traversal.len).map(|j| right.at(traversal.offset(i, j, k))).collect();
            data.push(Element::Nested(Array::from_parts(vec![traversal.len], cell, right.prototype().clone()).unwrap()));
        }
    }
    Array::from_parts(frame, data, Element::Nested(prototype)).map_err(|k| span.error(k, "invalid split result"))
}

fn partition(left: &Array, right: &Array, axis: Option<usize>, runs: bool, span: &Span) -> Result<Array, Error> {
    if left.shape().len() > 1 || runs && right.is_scalar() {
        return Err(span.error(ErrorKind::Rank, "partition needs a scalar/vector left argument and a non-scalar right argument"));
    }
    let right = if right.is_scalar() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(right.shape().len() - 1);
    let traversal = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid partition axis"))?;
    let counts: Vec<_> = left
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "partition marks must be nonnegative integers")))
        .collect::<Result<_, _>>()?;
    let extend = left.is_scalar() || runs && left.is_singleton();
    if !extend && (if runs { counts.len() != traversal.len } else { counts.len() > traversal.len.saturating_add(1) }) {
        return Err(span.error(ErrorKind::Length, "partition marks do not agree with the axis length"));
    }
    let len = if extend { traversal.len } else { counts.len() };
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut previous = 0;
    for j in 0..len {
        let count = counts[if extend { 0 } else { j }];
        let dividers = if runs { usize::from(count > previous) } else { count };
        if ranges.len().checked_add(dividers).is_none_or(|n| n > MAX_GENERATED_ELEMENTS) { return Err(span.error(ErrorKind::Limit, "too many partitions")); }
        for _ in 0..dividers { ranges.push(j..j); }
        if let Some(last) = ranges.last_mut() { if j < traversal.len && (!runs || count != 0) { last.end = j + 1; } }
        previous = count;
    }
    if !runs {
        if let Some(last) = ranges.last_mut() { last.end = traversal.len; }
        let mut shape = right.shape().to_vec();
        shape[axis] = 0;
        let prototype = Array::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid partition prototype"))?;
        let mut data = Vec::with_capacity(ranges.len());
        for range in ranges {
            let mut shape = right.shape().to_vec();
            shape[axis] = range.len();
            let width = range.len() * traversal.inner;
            let cell = remap(&right, shape, |i| Some(i / width * traversal.len * traversal.inner + range.start * traversal.inner + i % width), span)?;
            data.push(Element::Nested(cell));
        }
        return Array::from_parts(vec![data.len()], data, Element::Nested(prototype)).map_err(|k| span.error(k, "invalid partition result"));
    }
    let mut shape = right.shape().to_vec();
    shape[axis] = ranges.len();
    let size = generated_len(&shape).map_err(|k| span.error(k, "partition result is too large"))?;
    let prototype = Array::empty(vec![0], right.prototype().clone()).unwrap();
    let mut data = Vec::with_capacity(size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for range in &ranges {
            for k in 0..traversal.inner {
                let items = range.clone().map(|j| right.at(traversal.offset(i, j, k))).collect();
                let cell = Array::from_parts(vec![range.len()], items, right.prototype().clone()).map_err(|e| span.error(e, "invalid partition cell"))?;
                data.push(Element::Nested(cell));
            }
        }
    }
    Array::from_parts(shape, data, Element::Nested(prototype)).map_err(|k| span.error(k, "invalid partition result"))
}

fn squad(left: &Array, right: &Array, axis: Option<usize>, span: &Span) -> Result<Array, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "squad indices must be a scalar or vector")); }
    if left.len() > right.shape().len() || axis.is_some() && left.len() != 1 {
        return Err(span.error(ErrorKind::Length, "squad needs one index item per selected axis"));
    }
    let mut parts = vec![None; right.shape().len()];
    for (i, item) in left.elements().enumerate() {
        let coords = item.as_array();
        for n in coords.elements() { numeric(&n, span)?; }
        let part = parts.get_mut(axis.unwrap_or(i)).ok_or_else(|| span.error(ErrorKind::Rank, "squad axis is outside array rank"))?;
        *part = Some(coords);
    }
    select(right, &parts, span)
}

fn coordinate_offset(coords: &Array, right: &Array, span: &Span) -> Result<usize, Error> {
    if coords.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a coordinate must be a scalar or vector")); }
    if coords.len() != right.shape().len() { return Err(span.error(ErrorKind::Length, "a coordinate needs one index per axis")); }
    let mut offset = 0;
    for (n, &size) in coords.elements().zip(right.shape()) { offset = offset * size + index(numeric(&n, span)?, size, span)?; }
    Ok(offset)
}

fn pick(left: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a pick path must be a scalar or vector")); }
    let mut result = right.clone();
    for item in left.elements() {
        let offset = coordinate_offset(&item.as_array(), &result, span)?;
        result = result.at(offset).as_array();
    }
    Ok(result)
}

fn index(n: &Number, limit: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().map_err(|k| span.error(k, "index must be a positive integer"))?;
    if n <= 0 || n as usize > limit { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
    Ok(n as usize - 1)
}

pub(crate) fn select(right: &Array, parts: &[Option<Array>], span: &Span) -> Result<Array, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    if let [Some(indices)] = parts {
        if matches!(indices.elements().next().unwrap_or_else(|| indices.prototype().clone()), Element::Nested(_)) {
            let mut data = Vec::with_capacity(indices.len());
            for item in indices.elements() {
                let coords = item.as_array();
                if coords.shape().len() != 1 || coords.len() != right.shape().len() {
                    return Err(span.error(ErrorKind::Length, "a coordinate needs one index per axis"));
                }
                let offset = coordinate_offset(&coords, right, span)?;
                data.push(right.at(offset));
            }
            return Array::from_parts(indices.shape().to_vec(), data, right.prototype().clone()).map_err(|k| span.error(k, "invalid coordinate selection"));
        }
    }
    if parts.len() != right.shape().len() { return Err(span.error(ErrorKind::Rank, "one index expression is required per axis")); }
    let (mut shape, mut indices) = (Vec::new(), Vec::new());
    for (part, &size) in parts.iter().zip(right.shape()) {
        if let Some(a) = part {
            shape.extend_from_slice(a.shape());
            indices.push(Some(a.elements().map(|e| index(numeric(&e, span)?, size, span)).collect::<Result<Vec<_>, _>>()?));
        } else {
            shape.push(size);
            indices.push(None);
        }
    }
    remap(
        right,
        shape,
        |mut flat| {
            let (mut source, mut stride) = (0, 1);
            for axis in (0..parts.len()).rev() {
                let len = indices[axis].as_ref().map_or(right.shape()[axis], Vec::len);
                let coord = flat % len;
                flat /= len;
                source += indices[axis].as_ref().map_or(coord, |v| v[coord]) * stride;
                stride *= right.shape()[axis];
            }
            Some(source)
        },
        span,
    )
}
