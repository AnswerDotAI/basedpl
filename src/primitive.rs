use crate::{
    agreement::{Agreement, Mapping},
    array::{generated_len, Axis, MAX_GENERATED_ELEMENTS},
    execution::Context,
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
    At,
    Stencil,
}
impl Hybrid { pub(crate) fn primitive(self) -> Primitive { if self.scan { Primitive::Expand(self.first) } else { Primitive::Replicate(self.first) } } }

impl OperatorKind {
    pub(crate) fn glyph(self) -> &'static str {
        use OperatorKind::*;
        match self {
            Each => "¨",
            Commute => "⍨",
            Compose => "∘",
            Rank => "⍤",
            Over => "⍥",
            Behind => "⍛",
            Product => ".",
            Outer => "∘.",
            Key => "⌸",
            Power => "⍣",
            At => "@",
            Stencil => "⌺",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
impl Comparison {
    fn ordered(self, order: Ordering) -> bool {
        match self {
            Self::Equal => order.is_eq(),
            Self::NotEqual => !order.is_eq(),
            Self::Less => order.is_lt(),
            Self::LessEqual => !order.is_gt(),
            Self::Greater => order.is_gt(),
            Self::GreaterEqual => !order.is_lt(),
        }
    }
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
    Encode,
    Decode,
    MatrixDivide,
    Execute,
    Format,
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
fn integer(n: i64) -> Element { Element::Number(Number::from_integer(n)) }
fn generated(n: usize, exact: bool) -> Element {
    if !exact { return float(n as f64); }
    match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Element::Number(Number::try_from(num_rational::BigRational::from_integer(n.into())).unwrap()) }
}
fn selected(a: &Array, i: usize) -> Element { a.at(if a.is_singleton() { 0 } else { i }) }

pub(crate) fn axis_value(axis: usize) -> Array { Array::scalar(Number::from_integer((axis + 1) as i64)).unwrap() }
pub(crate) fn single_axis(axis: &Array, span: &Span) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "one axis is required")); }
    numeric(&axis.at(0), span)?
        .nonnegative_integer()
        .map_err(|k| span.error(k, "axis must be integral"))?
        .checked_sub(1)
        .ok_or_else(|| span.error(ErrorKind::Domain, "axes start at one"))
}

fn axes(axis: &Array, rank: usize, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be scalar or vector")); }
    let mut result = Vec::new();
    for e in axis.elements() {
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "axis must be integral"))?;
        if n == 0 || n > rank || result.contains(&(n - 1)) { return Err(span.error(ErrorKind::Domain, "axes must be distinct and within the array rank")); }
        result.push(n - 1);
    }
    Ok(result)
}

fn fractional_axis(axis: &Array, rank: usize, span: &Context<'_>) -> Result<Option<usize>, Error> {
    if axis.shape().len() > 1 || !axis.is_singleton() { return Ok(None); }
    let n = numeric(&axis.at(0), span)?.clone();
    if n.integer().is_ok() { return Ok(None); }
    let z = n.to_complex().map_err(|m| span.error(ErrorKind::Domain, m))?;
    if z.im != 0. || z.re <= 0. || z.re >= (rank + 1) as f64 { return Err(span.error(ErrorKind::Domain, "fractional axis is outside the array rank")); }
    Ok(Some(z.re.floor() as usize))
}

fn float_binary<T>(x: &[f64], y: &[f64], agreement: &Agreement, f: impl Fn(f64, f64) -> T) -> Vec<T> {
    match (&agreement.left, &agreement.right) {
        (Mapping::Scalar, Mapping::Linear(1)) => y.iter().map(|&b| f(x[0], b)).collect(),
        (Mapping::Linear(1), Mapping::Scalar) => x.iter().map(|&a| f(a, y[0])).collect(),
        (Mapping::Linear(1), Mapping::Linear(1)) => x.iter().zip(y).map(|(&a, &b)| f(a, b)).collect(),
        _ => (0..agreement.len).map(|i| f(x[agreement.left.index(i)], y[agreement.right.index(i)])).collect(),
    }
}

fn float_apply(op: Primitive, left: Option<&[f64]>, right: &[f64], agreement: &Agreement, span: &Context<'_>) -> Result<Array, Error> {
    use crate::number::float_equal;
    use Arithmetic::*;
    use Comparison::*;
    let data = match (op, left) {
        (Primitive::Arithmetic(op), Some(x)) => match op {
            Plus => float_binary(x, right, agreement, |a, b| a + b),
            Minus => float_binary(x, right, agreement, |a, b| a - b),
            Times => float_binary(x, right, agreement, |a, b| a * b),
            Divide => float_binary(x, right, agreement, |a, b| if a == 0.0 && b == 0.0 { 1.0 } else { a / b }),
        },
        (Primitive::Arithmetic(op), None) => match op {
            Plus => right.to_vec(),
            Minus => right.iter().map(|a| -a).collect(),
            Times => right.iter().map(|&a| if a == 0.0 { 0.0 } else { a.signum() }).collect(),
            Divide => right.iter().map(|a| 1.0 / a).collect(),
        },
        (Primitive::Compare(op), Some(x)) => {
            let boolean = |b| i64::from(b);
            let data = match op {
                Equal => float_binary(x, right, agreement, |a, b| boolean(float_equal(a, b))),
                NotEqual => float_binary(x, right, agreement, |a, b| boolean(!float_equal(a, b))),
                Less => float_binary(x, right, agreement, |a, b| boolean(a < b && !float_equal(a, b))),
                Greater => float_binary(x, right, agreement, |a, b| boolean(a > b && !float_equal(a, b))),
                LessEqual => float_binary(x, right, agreement, |a, b| boolean(a < b || float_equal(a, b))),
                GreaterEqual => float_binary(x, right, agreement, |a, b| boolean(a > b || float_equal(a, b))),
            };
            return Array::integers(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid comparison result"));
        }
        _ => unreachable!(),
    };
    Array::floats(agreement.shape.clone(), data).map_err(|k| {
        let zero_divisor = matches!(op, Primitive::Arithmetic(Divide))
            && (0..agreement.len).any(|i| right[agreement.right.index(i)] == 0.0 && left.is_none_or(|x| x[agreement.left.index(i)] != 0.0));
        span.error(k, if zero_divisor { "division by zero" } else { "result is not finite" })
    })
}

/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    pub(crate) fn glyph(self) -> char {
        use Arithmetic::*;
        use Comparison::*;
        use Math::*;
        match self {
            Self::Arithmetic(p) => match p {
                Plus => '+',
                Minus => '-',
                Times => '×',
                Divide => '÷',
            },
            Self::Math(p) => match p {
                Magnitude => '|',
                Floor => '⌊',
                Ceiling => '⌈',
                Power => '*',
                Log => '⍟',
                Circle => '○',
                Factorial => '!',
                Gcd => '∨',
                Lcm => '∧',
                Nand => '⍲',
                Nor => '⍱',
                Not => '~',
            },
            Self::Compare(p) => match p {
                Equal => '=',
                NotEqual => '≠',
                Less => '<',
                LessEqual => '≤',
                Greater => '>',
                GreaterEqual => '≥',
            },
            Self::Random => '?',
            Self::Identity(true) => '⊣',
            Self::Identity(false) => '⊢',
            Self::Iota => '⍳',
            Self::Depth => '≡',
            Self::Where => '⍸',
            Self::Member => '∊',
            Self::Union => '∪',
            Self::Intersection => '∩',
            Self::Find => '⍷',
            Self::Grade(false) => '⍋',
            Self::Grade(true) => '⍒',
            Self::Index => '⌷',
            Self::Encode => '⊤',
            Self::Decode => '⊥',
            Self::MatrixDivide => '⌹',
            Self::Execute => '⍎',
            Self::Format => '⍕',
            Self::Shape => '⍴',
            Self::Tally => '≢',
            Self::Ravel => ',',
            Self::Enclose => '⊂',
            Self::Nest => '⊆',
            Self::Disclose => '⊃',
            Self::Take => '↑',
            Self::Drop => '↓',
            Self::Replicate(false) => '/',
            Self::Replicate(true) => '⌿',
            Self::Expand(false) => '\\',
            Self::Expand(true) => '⍀',
            Self::CatenateFirst => '⍪',
            Self::Reverse(false) => '⌽',
            Self::Reverse(true) => '⊖',
            Self::Transpose => '⍉',
        }
    }
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
            '⊤' => Self::Encode,
            '⊥' => Self::Decode,
            '⌹' => Self::MatrixDivide,
            '⍎' => Self::Execute,
            '⍕' => Self::Format,
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
    pub(crate) fn call(self, left: Option<&Array>, right: &Array, span: &Context<'_>) -> Result<Array, Error> { self.call_axis(left, right, None, span) }
    pub(crate) fn call_axes(self, left: Option<&Array>, right: &Array, spec: &Array, span: &Context<'_>) -> Result<Array, Error> {
        if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be scalar or vector")); }
        if let Some(left) = left {
            if matches!(self, Self::Arithmetic(_) | Self::Math(_) | Self::Compare(_)) && !matches!(self, Self::Math(Math::Not)) {
                return scalar_axes(self, left, right, spec, span);
            }
            if matches!(self, Self::Ravel | Self::CatenateFirst) {
                if let Some(axis) = fractional_axis(spec, right.shape().len().max(left.shape().len()), span)? { return laminate(left, right, axis, span); }
            }
        }
        match (self, left) {
            (Self::Ravel, None) => {
                let mut shape = right.shape().to_vec();
                if spec.is_empty() { shape.push(1); }
                else if let Some(axis) = fractional_axis(spec, shape.len(), span)? { shape.insert(axis, 1); }
                else {
                    let axes = axes(spec, shape.len(), span)?;
                    if axes.windows(2).any(|a| a[1] != a[0] + 1) { return Err(span.error(ErrorKind::Domain, "ravel axes must be consecutive and ascending")); }
                    let start = axes[0];
                    let len = crate::array::element_count(&shape[start..start + axes.len()]).map_err(|k| span.error(k, "ravel shape overflow"))?;
                    shape.splice(start..start + axes.len(), [len]);
                }
                right.with_shape(shape).map_err(|k| span.error(k, "invalid ravel shape"))
            }
            (Self::Enclose, None) => enclose_axes(right, &axes(spec, right.shape().len(), span)?, span),
            (Self::Take, None) => mix_axes(right, spec, span),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, Some(&axes(spec, right.shape().len(), span)?), span),
            (Self::Index, Some(x)) => squad(x, right, Some(&axes(spec, right.shape().len(), span)?), span),
            (Self::CatenateFirst, None) => Err(span.error(ErrorKind::Syntax, "table does not accept an axis")),
            _ => self.call_axis(left, right, Some(single_axis(spec, span)?), span),
        }
    }
    pub(crate) fn call_axis(self, left: Option<&Array>, right: &Array, axis: Option<usize>, span: &Context<'_>) -> Result<Array, Error> {
        span.check()?;
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
        { return Err(span.error(ErrorKind::Syntax, "axis is not supported by this primitive")); }
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
            Self::MatrixDivide => return matrix_divide(left, right, span),
            Self::Format => return format_array(left, right, span),
            Self::Encode | Self::Decode => {
                return radix(
                    left.ok_or_else(|| span.error(ErrorKind::Syntax, "encode/decode needs a left argument"))?,
                    right,
                    matches!(self, Self::Encode),
                    span,
                )
            }
            Self::Index => {
                return match left { Some(x) => squad(x, right, axis.map(|a| vec![a]).as_deref(), span), None => Ok(right.clone()) }
            }
            Self::Math(Math::Not) if left.is_some() => return without(left.unwrap(), right, span),
            Self::Depth => {
                return match left {
                    Some(x) => construct(vec![], vec![integer(i64::from(array_match(x, right, span)?))]),
                    None => construct(vec![], vec![float(depth(right) as f64)]),
                }
            }
            Self::Tally if left.is_some() => return construct(vec![], vec![integer(i64::from(!array_match(left.unwrap(), right, span)?))]),
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
                if axis.is_some() { return Err(span.error(ErrorKind::Syntax, "table does not take an axis")); }
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
                if let Some(axis) = axis {
                    if matches!(self, Self::Nest) { return Err(span.error(ErrorKind::Syntax, "nest does not take an axis")); }
                    return self.call_axes(None, right, &axis_value(axis), span);
                }
                if matches!(self, Self::Nest) && right.elements().chain(std::iter::once(right.prototype().clone())).any(|e| matches!(e, Element::Nested(_))) {
                    return Ok(right.clone());
                }
                return Array::new(vec![], vec![Element::Nested(right.clone())]).map_err(|k| span.error(k, "invalid enclosure"));
            }
            Self::Take | Self::Drop => {
                if left.is_none() && matches!(self, Self::Take) {
                    if let Some(axis) = axis { return mix_axes(right, &axis_value(axis), span); }
                    let cells: Vec<_> = right.elements().map(|e| e.as_array()).collect();
                    return Array::assemble(right.shape(), &cells, &right.prototype().as_array()).map_err(|k| span.error(k, "cannot assemble cells"));
                }
                if left.is_none() { return split(right, axis, span); }
                return take_drop(matches!(self, Self::Take), left.unwrap(), right, axis.map(|a| vec![a]).as_deref(), span);
            }
            Self::Shape | Self::Tally | Self::Ravel => {
                return match self {
                    Self::Shape => Array::from_parts(
                        vec![right.shape().len()],
                        right.shape().iter().map(|&n| generated(n, right.is_exact())).collect(),
                        generated(0, right.is_exact()),
                    )
                    .map_err(|k| span.error(k, "invalid shape")),
                    Self::Tally => construct(vec![], vec![generated(right.shape().first().copied().unwrap_or(1), right.is_exact())]),
                    Self::Ravel => {
                        if let Some(axis) = axis { return self.call_axes(None, right, &axis_value(axis), span); }
                        right.with_shape(vec![right.len()]).map_err(|k| span.error(k, "invalid ravel"))
                    }
                    _ => unreachable!(),
                };
            }
            _ => (),
        }
        if matches!(self, Self::Compare(_)) && left.is_none() { return Err(span.error(ErrorKind::Syntax, "this primitive needs a left argument")); }
        self.scalar_apply(left, right, span, false)
    }

    fn scalar_apply(self, left: Option<&Array>, right: &Array, span: &Context<'_>, fill: bool) -> Result<Array, Error> {
        if matches!(self, Self::Random) && !fill { return roll(right, right.is_exact(), span, false); }
        let agreement = Agreement::new(left.map_or(&[], Array::shape), right.shape()).map_err(|k| span.error(k, "array shapes do not agree"))?;
        self.scalar_mapped(left, right, span, fill, &agreement)
    }
    fn scalar_mapped(self, left: Option<&Array>, right: &Array, span: &Context<'_>, fill: bool, agreement: &Agreement) -> Result<Array, Error> {
        if agreement.len == 0 {
            let prototype = self.scalar_item(left.map(Array::prototype), right.prototype(), span, true)?;
            return Array::empty(agreement.shape.clone(), prototype).map_err(|k| span.error(k, "invalid empty result"));
        }
        if matches!(self, Self::Arithmetic(_) | Self::Compare(_)) && right.as_floats().is_some() && left.is_none_or(|a| a.as_floats().is_some()) {
            return float_apply(self, left.map(|a| a.as_floats().unwrap()), right.as_floats().unwrap(), agreement, span);
        }
        if let (Self::Compare(op), Some(x), Some(y)) = (self, left.and_then(Array::as_integers), right.as_integers()) {
            let data = (0..agreement.len).map(|i| i64::from(op.ordered(x[agreement.left.index(i)].cmp(&y[agreement.right.index(i)])))).collect();
            return Array::integers(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid comparison result"));
        }
        if let Self::Arithmetic(op) = self {
            if let Some(y) = right.as_integers() {
                if left.is_none_or(|a| a.as_integers().is_some()) {
                    let x = left.map(|a| a.as_integers().unwrap());
                    let data = (0..agreement.len)
                        .map(|i| Number::checked_integer(op, x.map(|v| v[agreement.left.index(i)]), y[agreement.right.index(i)]))
                        .collect::<Option<Vec<_>>>();
                    if let Some(data) = data { return Array::integers(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid integer result")); }
                }
            }
        }
        let data = (0..agreement.len)
            .map(|i| self.scalar_item(left.map(|a| a.at(agreement.left.index(i))).as_ref(), &right.at(agreement.right.index(i)), span, fill))
            .collect::<Result<_, _>>()?;
        Array::new(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid scalar result"))
    }

    fn scalar_item(self, left: Option<&Element>, right: &Element, span: &Context<'_>, fill: bool) -> Result<Element, Error> {
        span.check()?;
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Element::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Element::Nested(_)) || matches!(left, Some(Element::Nested(_))) {
            return self.scalar_apply(left.map(|e| e.as_array()).as_ref(), &right.as_array(), span, fill).map(Element::Nested);
        }
        if fill {
            return Ok(match (self, left, right) {
                (Self::Compare(_) | Self::Math(Math::Not | Math::Nand | Math::Nor), _, _) => integer(0),
                (Self::Random, _, Element::Number(y)) => Element::Number(y.unit(0)),
                (Self::Math(Math::Circle | Math::Log), _, _) | (Self::Math(Math::Power), None, _) => float(0.0),
                (Self::Math(_), None, Element::Number(y)) => Element::Number(y.result_zero(None)),
                (Self::Math(_), Some(Element::Number(x)), Element::Number(y)) => Element::Number(y.result_zero(Some(x))),
                (Self::Arithmetic(_), None, Element::Number(y)) => Element::Number(y.result_zero(None)),
                (Self::Arithmetic(_), Some(Element::Number(x)), Element::Number(y)) => Element::Number(y.result_zero(Some(x))),
                _ => float(0.0),
            });
        }
        match self {
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
                    _ => numeric(x, span)?.compare(numeric(right, span)?).map(|order| op.ordered(order)),
                };
                result.map(|b| integer(i64::from(b))).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            _ => unreachable!(),
        }
    }
}

pub(crate) fn array_match(left: &Array, right: &Array, span: &Context<'_>) -> Result<bool, Error> {
    span.check()?;
    if left.shape() != right.shape() { return Ok(false); }
    if left.is_empty() { return element_match(left.prototype(), right.prototype(), span); }
    for (x, y) in left.elements().zip(right.elements()) { if !element_match(&x, &y, span)? { return Ok(false); } }
    Ok(true)
}

fn element_match(left: &Element, right: &Element, span: &Context<'_>) -> Result<bool, Error> {
    span.check()?;
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

fn without(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "without needs a scalar or vector left argument")); }
    let mut data = Vec::new();
    for x in left.elements() { if !contains(right, &x, span)? { data.push(x); } }
    Array::from_parts(vec![data.len()], data, left.prototype().clone()).map_err(|k| span.error(k, "invalid without result"))
}

fn contains(array: &Array, element: &Element, span: &Context<'_>) -> Result<bool, Error> {
    for item in array.elements() { if element_match(&item, element, span)? { return Ok(true); } }
    Ok(false)
}

fn membership(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let data = left.elements().map(|e| contains(right, &e, span).map(i64::from)).collect::<Result<_, _>>()?;
    Array::integers(left.shape().to_vec(), data).map_err(|k| span.error(k, "invalid membership result"))
}

fn enlist(right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    fn append(array: &Array, data: &mut Vec<Element>, span: &Context<'_>) -> Result<(), Error> {
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

fn unique(right: &Array, span: &Context<'_>) -> Result<Array, Error> { replicate(&unique_mask(right, span)?, right, true, None, false, span) }

fn union(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if left.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "union needs scalars or vectors")); }
    catenate(left, &without(right, left, span)?, None, false, span)
}

fn intersection(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if left.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "intersection needs scalars or vectors")); }
    replicate(&membership(left, right, span)?, left, true, None, false, span)
}

fn find(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let mut pattern_shape = vec![1; right.shape().len().saturating_sub(left.shape().len())];
    pattern_shape.extend_from_slice(left.shape());
    let mut data = vec![integer(0); right.len()];
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
            *result = integer(i64::from(matched));
        }
    }
    Array::from_parts(right.shape().to_vec(), data, integer(0)).map_err(|k| span.error(k, "invalid find result"))
}

fn unique_mask(right: &Array, span: &Context<'_>) -> Result<Array, Error> {
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
        data.push(integer(i64::from(unique)));
    }
    Array::from_parts(vec![data.len()], data, integer(0)).map_err(|k| span.error(k, "invalid unique mask"))
}

fn coordinates(shape: &[usize], mut flat: usize, exact: bool) -> Array {
    let mut data = vec![generated(0, exact); shape.len()];
    for axis in (0..shape.len()).rev() {
        data[axis] = generated(flat % shape[axis] + 1, exact);
        flat /= shape[axis];
    }
    Array::from_parts(vec![data.len()], data, generated(0, exact)).unwrap()
}

fn iota(right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "iota needs a scalar or vector shape")); }
    let shape = right
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid iota dimension")))
        .collect::<Result<Vec<_>, _>>()?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "iota exceeds array limits"))?;
    let exact = right.is_exact();
    if right.is_singleton() {
        return if exact { Array::integers(shape, (1..=len).map(|i| i as i64).collect()) } else { Array::floats(shape, (1..=len).map(|i| i as f64).collect()) }
            .map_err(|k| span.error(k, "invalid iota"));
    }
    let zero = generated(0, exact);
    let prototype = Element::Nested(Array::from_parts(vec![shape.len()], vec![zero.clone(); shape.len()], zero).unwrap());
    let data = (0..len).map(|i| Element::Nested(coordinates(&shape, i, exact))).collect();
    Array::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid coordinate array"))
}

fn where_indices(right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let mut data = Vec::new();
    for (i, e) in right.elements().enumerate() {
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "where needs nonnegative integer counts"))?;
        if n > MAX_GENERATED_ELEMENTS - data.len() { return Err(span.error(ErrorKind::Limit, "where exceeds array limits")); }
        let index = if right.shape().len() == 1 { generated(i + 1, true) } else { Element::Nested(coordinates(right.shape(), i, true)) };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype =
        if right.shape().len() == 1 { integer(0) } else { Element::Nested(Array::integers(vec![right.shape().len()], vec![0; right.shape().len()]).unwrap()) };
    Array::from_parts(vec![data.len()], data, prototype).map_err(|k| span.error(k, "invalid where result"))
}

struct SearchCells { left: Vec<Array>, right: Vec<Array>, shape: Vec<usize> }

fn search_cells(left: &Array, right: &Array, span: &Context<'_>) -> Result<SearchCells, Error> {
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

fn index_of(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
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
        data.push(generated(found + 1, true));
    }
    Array::from_parts(shape, data, integer(0)).map_err(|k| span.error(k, "invalid index-of result"))
}

fn format_number(n: &Number, precision: isize, span: &Context<'_>) -> Result<String, Error> {
    use num_traits::Signed;
    if n.as_complex().is_some() { return Err(span.error(ErrorKind::Domain, "specified format requires real numbers")); }
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
        let y = n.to_complex().map_err(|m| span.error(ErrorKind::Domain, m))?.re;
        if precision >= 0 { format!("{y:.digits$}") } else {
            let s = format!("{:.*e}", digits - 1, y);
            let (mantissa, exponent) = s.split_once('e').unwrap();
            format!("{mantissa}E{}", exponent.parse::<i32>().unwrap())
        }
    };
    if !n.is_exact() {
        let mut significant = 0;
        let mut exponent = false;
        text = text
            .chars()
            .map(|c| {
                if c == 'E' { exponent = true; }
                if !exponent && c.is_ascii_digit() && (c != '0' || significant != 0) { significant += 1; }
                if !exponent && c.is_ascii_digit() && significant > 16 { '_' } else { c }
            })
            .collect();
    }
    Ok(text.replace('-', "¯"))
}

fn format_array(left: Option<&Array>, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let Some(spec) = left else { return right.formatted().map_err(|k| span.error(k, "formatted array is too large")); };
    if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "format specification must be scalar or vector")); }
    let spec = spec
        .elements()
        .map(|e| numeric(&e, span)?.integer().map_err(|k| span.error(k, "format specification must be integral")))
        .collect::<Result<Vec<_>, _>>()?;
    let columns = right.shape().last().copied().unwrap_or(1);
    if !matches!(spec.len(), 1 | 2) && spec.len() != 2 * columns {
        return Err(span.error(ErrorKind::Length, "format needs precision, a width/precision pair, or pairs per column"));
    }
    numeric(right.prototype(), span)?;
    let specs: Vec<_> = (0..columns)
        .map(|i| match spec.len() { 1 => (0, spec[0]), 2 => (spec[0], spec[1]), _ => (spec[2 * i], spec[2 * i + 1]) })
        .collect();
    let mut widths = Vec::new();
    for &(width, precision) in &specs {
        if width < 0 { return Err(span.error(ErrorKind::Domain, "format width must be nonnegative")); }
        generated_len(&[right.len().max(1), precision.unsigned_abs().max(width as usize).max(1)]).map_err(|k| span.error(k, "format exceeds array limits"))?;
        widths.push(if width == 0 { 1 } else { width as usize });
    }
    let text = right
        .elements()
        .enumerate()
        .map(|(i, e)| {
            let s = format_number(numeric(&e, span)?, specs[i % columns].1, span)?;
            if specs[i % columns].0 == 0 { widths[i % columns] = widths[i % columns].max(s.chars().count() + 1); }
            Ok(s)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let width = widths.iter().sum();
    let mut shape = right.shape().to_vec();
    if let Some(last) = shape.last_mut() { *last = width; }
    else { shape.push(width); }
    generated_len(&shape).map_err(|k| span.error(k, "formatted result is too large"))?;
    let mut data = Vec::new();
    for (i, s) in text.iter().enumerate() {
        let width = widths[i % columns];
        let len = s.chars().count();
        let s = if len > width { "*".repeat(width) } else if specs[i % columns].1 < 0 { format!("{s}{}", " ".repeat(width - len)) } else { format!("{}{s}", " ".repeat(width - len)) };
        data.extend(s.chars().map(Element::Character));
    }
    Array::from_parts(shape, data, Element::Character(' ')).map_err(|k| span.error(k, "invalid formatted result"))
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Array, bool)>, right: &Array, axis: Option<usize>, span: &Context<'_>) -> Result<Array, Error> {
    use crate::number::Arithmetic::*;
    use Primitive::*;
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide)) && a.elements().any(|e| matches!(e, Element::Number(n) if n.equal(&n.unit(0)).unwrap_or(false))) {
            return Err(span.error(ErrorKind::Domain, "zero multiplier/divisor has no inverse"));
        }
        return match p {
            Arithmetic(Plus) => Arithmetic(Minus).call(Some(right), a, span),
            Arithmetic(Minus) if first => p.call(Some(a), right, span),
            Arithmetic(Minus) => Arithmetic(Plus).call(Some(right), a, span),
            Arithmetic(Times) => Arithmetic(Divide).call(Some(right), a, span),
            Arithmetic(Divide) if first => p.call(Some(a), right, span),
            Arithmetic(Divide) => Arithmetic(Times).call(Some(right), a, span),
            Math(crate::number::Math::Power) if first => Math(crate::number::Math::Log).call(Some(a), right, span),
            Math(crate::number::Math::Power) => p.call(Some(right), &Arithmetic(Divide).call(None, a, span)?, span),
            Math(crate::number::Math::Log) if first => Math(crate::number::Math::Power).call(Some(a), right, span),
            Math(crate::number::Math::Log) => Math(crate::number::Math::Power).call(Some(a), &Arithmetic(Divide).call(None, right, span)?, span),
            Math(crate::number::Math::Circle) if first => {
                let codes = a
                    .elements()
                    .map(|e| {
                        let code = numeric(&e, span)?.integer().map_err(|k| span.error(k, "circle inverse needs integer codes"))?;
                        if !(-7..=7).contains(&code) { return Err(span.error(ErrorKind::Domain, "circle code has no supported inverse")); }
                        Ok(integer(-code as i64))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                let codes = Array::from_parts(a.shape().to_vec(), codes, integer(0)).map_err(|k| span.error(k, "invalid circle codes"))?;
                p.call(Some(&codes), right, span)
            }
            Reverse(_) if first => p.call_axis(Some(&Arithmetic(Minus).call(None, a, span)?), right, axis, span),
            Transpose if first => {
                let perm = axes(a, right.shape().len(), span)?;
                if perm.len() != right.shape().len() { return Err(span.error(ErrorKind::Length, "inverse transpose needs an axis permutation")); }
                let mut inverse = vec![0; perm.len()];
                for (i, &axis) in perm.iter().enumerate() { inverse[axis] = i as i64 + 1; }
                p.call(Some(&Array::integers(vec![inverse.len()], inverse).unwrap()), right, span)
            }
            Decode if first => inverse_decode(a, right, span),
            Encode if first => Decode.call(Some(a), right, span),
            _ => Err(span.error(ErrorKind::Domain, "this bound function has no known inverse")),
        };
    }
    match p {
        Arithmetic(Plus | Minus | Divide) | Reverse(_) | Transpose | Identity(_) | Index | MatrixDivide => p.call_axis(None, right, axis, span),
        Math(crate::number::Math::Power) => Math(crate::number::Math::Log).call(None, right, span),
        Math(crate::number::Math::Log) => Math(crate::number::Math::Power).call(None, right, span),
        Math(crate::number::Math::Circle) => Arithmetic(Divide).call(Some(right), &Array::scalar(std::f64::consts::PI).unwrap(), span),
        Enclose => Disclose.call(None, right, span),
        Disclose => Enclose.call(None, right, span),
        Take => split(right, axis, span),
        Drop => Take.call(None, right, span),
        Where => inverse_where(right, span),
        _ => Err(span.error(ErrorKind::Domain, "this primitive has no known inverse")),
    }
}

fn inverse_decode(base: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if !base.is_scalar() {
        if base.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "inverse decode needs a scalar or vector base")); }
        let result = radix(base, right, true, span)?;
        if !array_match(&radix(base, &result, false, span)?, right, span)? {
            return Err(span.error(ErrorKind::Domain, "value cannot be represented in these bases"));
        }
        return Ok(result);
    }
    let b = numeric(&base.at(0), span)?.clone();
    if b.order(&b.unit(1)).map_err(|m| span.error(ErrorKind::Domain, m))? != Ordering::Greater {
        return Err(span.error(ErrorKind::Domain, "inverse decode base must exceed one"));
    }
    let mut digits = 0;
    for e in right.elements() {
        let mut n = numeric(&e, span)?.clone();
        if n.order(&n.unit(0)).map_err(|m| span.error(ErrorKind::Domain, m))?.is_lt() {
            return Err(span.error(ErrorKind::Domain, "inverse decode requires nonnegative values"));
        }
        let mut count = 0;
        while !n.equal(&n.unit(0)).map_err(|m| span.error(ErrorKind::Domain, m))? {
            span.check()?;
            if n.order(&n.unit(1)).map_err(|m| span.error(ErrorKind::Domain, m))?.is_lt() {
                return Err(span.error(ErrorKind::Domain, "inverse decode cannot represent this value"));
            }
            n = n.dyad(Arithmetic::Divide, &b).and_then(|n| n.math_monad(Math::Floor)).map_err(|m| span.error(ErrorKind::Domain, m))?;
            count += 1;
            if count > MAX_GENERATED_ELEMENTS { return Err(span.error(ErrorKind::Limit, "inverse decode is too large")); }
        }
        digits = digits.max(count);
    }
    let bases = Array::from_parts(vec![digits], vec![Element::Number(b); digits], base.prototype().clone())
        .map_err(|k| span.error(k, "invalid inverse decode base"))?;
    radix(&bases, right, true, span)
}

fn inverse_where(right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.as_array();
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate =
            a.elements().map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid position"))).collect::<Result<Vec<_>, _>>()?;
        if coordinate.contains(&0) { return Err(span.error(ErrorKind::Domain, "positions start at one")); }
        if coordinates.is_empty() { shape.resize(coordinate.len(), 0); }
        if coordinate.len() != shape.len() { return Err(span.error(ErrorKind::Length, "coordinate lengths differ")); }
        if coordinates.last().is_some_and(|previous| previous > &coordinate) {
            return Err(span.error(ErrorKind::Domain, "inverse where positions must be sorted"));
        }
        for (size, &c) in shape.iter_mut().zip(&coordinate) { *size = (*size).max(c); }
        coordinates.push(coordinate);
    }
    if coordinates.is_empty() { shape = vec![0]; }
    let mut data = vec![0i64; generated_len(&shape).map_err(|k| span.error(k, "inverse where is too large"))?];
    for coordinate in coordinates {
        let index = coordinate.iter().zip(&shape).fold(0, |i, (&c, &d)| i * d + c - 1);
        data[index] += 1;
    }
    Array::integers(shape, data).map_err(|k| span.error(k, "invalid inverse where"))
}

fn matrix_divide(left: Option<&Array>, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    use faer::{linalg::solvers::SolveLstsq, Mat};
    let dimensions = |a: &Array| match a.shape() {
        [] => Ok((1, 1)),
        &[m] => Ok((m, 1)),
        &[m, n] => Ok((m, n)),
        _ => Err(span.error(ErrorKind::Rank, "matrix divide requires rank at most two")),
    };
    let (m, n) = dimensions(right)?;
    let k = match left {
        Some(x) => {
            let (rows, columns) = dimensions(x)?;
            if rows != m { return Err(span.error(ErrorKind::Length, "matrix row counts differ")); }
            columns
        }
        None => m,
    };
    if m < n { return Err(span.error(ErrorKind::Domain, "matrix is underdetermined")); }
    let mut shape = right.shape().get(1..).unwrap_or(&[]).to_vec();
    if let Some(x) = left { shape.extend_from_slice(x.shape().get(1..).unwrap_or(&[])); }
    else { shape = right.shape().iter().rev().copied().collect(); }
    generated_len(&[n, k]).map_err(|e| span.error(e, "matrix result is too large"))?;
    let numbers = |a: &Array| -> Result<Vec<Number>, Error> {
        numeric(a.prototype(), span)?;
        a.elements().map(|e| numeric(&e, span).cloned()).collect()
    };
    let a = numbers(right)?;
    let b = left.map(numbers).transpose()?;
    let exact = right.is_exact() && left.is_none_or(Array::is_exact);
    let prototype = if exact { right.prototype().clone() } else { float(0.) };
    if n == 0 { return Array::empty(shape, prototype).map_err(|e| span.error(e, "invalid matrix shape")); }
    let values = if exact { exact_solve(&a, b.as_deref(), m, n, k, span)? } else {
        let convert = |v: &[Number]| v.iter().map(|x| x.to_complex().map_err(|e| span.error(ErrorKind::Domain, e))).collect::<Result<Vec<_>, _>>();
        let a = convert(&a)?;
        let matrix = Mat::from_fn(m, n, |i, j| a[i * n + j]);
        let svd = matrix.thin_svd().map_err(|_| span.error(ErrorKind::Domain, "matrix factorization failed"))?;
        let cutoff = f64::EPSILON * m.max(n) as f64 * svd.S()[0].re;
        if svd.S()[n - 1].re <= cutoff { return Err(span.error(ErrorKind::Domain, "matrix is rank deficient")); }
        let result = match b {
            Some(b) => {
                let b = convert(&b)?;
                svd.solve_lstsq(Mat::from_fn(m, k, |i, j| b[i * k + j]))
            }
            None => svd.pseudoinverse(),
        };
        (0..n)
            .flat_map(|i| (0..k).map(move |j| (i, j)))
            .map(|(i, j)| Number::try_from(result[(i, j)]).map_err(|e| span.error(e, "matrix result is not finite")))
            .collect::<Result<Vec<_>, _>>()?
    };
    Array::from_parts(shape, values.into_iter().map(Element::Number).collect(), prototype).map_err(|e| span.error(e, "invalid matrix result"))
}

// Normal equations are exact here; the approximate path never forms AᵀA.
fn exact_solve(a: &[Number], b: Option<&[Number]>, m: usize, n: usize, k: usize, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    use num_rational::BigRational;
    use num_traits::{One, Zero};
    let a: Vec<_> = a.iter().map(|x| x.as_exact().unwrap()).collect();
    let b = b.map(|v| v.iter().map(|x| x.as_exact().unwrap()).collect::<Vec<_>>());
    let mut coefficients = if m == n { a.clone() } else { (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).map(|(i, j)| (0..m).map(|r| &a[r * n + i] * &a[r * n + j]).sum()).collect() };
    let mut rhs = (0..n)
        .flat_map(|i| (0..k).map(move |j| (i, j)))
        .map(|(i, j)| match &b {
            Some(b) if m == n => b[i * k + j].clone(),
            Some(b) => (0..m).map(|r| &a[r * n + i] * &b[r * k + j]).sum(),
            None if m == n => {
                if i == j { BigRational::one() } else { BigRational::zero() }
            }
            None => a[j * n + i].clone(),
        })
        .collect::<Vec<_>>();
    for p in 0..n {
        span.check()?;
        let row = (p..n).find(|&r| !coefficients[r * n + p].is_zero()).ok_or_else(|| span.error(ErrorKind::Domain, "matrix is rank deficient"))?;
        for j in 0..n { coefficients.swap(p * n + j, row * n + j); }
        for j in 0..k { rhs.swap(p * k + j, row * k + j); }
        let pivot = coefficients[p * n + p].clone();
        for j in p..n { coefficients[p * n + j] /= &pivot; }
        for j in 0..k { rhs[p * k + j] /= &pivot; }
        for i in 0..n {
            span.check()?;
            if i == p { continue; }
            let factor = coefficients[i * n + p].clone();
            if factor.is_zero() { continue; }
            for j in p..n {
                let change = &factor * &coefficients[p * n + j];
                coefficients[i * n + j] -= change;
            }
            for j in 0..k {
                let change = &factor * &rhs[p * k + j];
                rhs[i * k + j] -= change;
            }
        }
    }
    Ok(rhs.into_iter().map(|x| Number::try_from(x).expect("canonical rational solution")).collect())
}

fn radix(left: &Array, right: &Array, encode: bool, span: &Context<'_>) -> Result<Array, Error> {
    let numbers = |a: &Array| a.elements().map(|e| numeric(&e, span).cloned()).collect::<Result<Vec<_>, _>>();
    let (xs, ys) = (numbers(left)?, numbers(right)?);
    let zero = numeric(right.prototype(), span)?.result_zero(Some(numeric(left.prototype(), span)?));
    let error = |m| span.error(ErrorKind::Domain, m);
    if encode {
        let shape = [left.shape(), right.shape()].concat();
        let count = generated_len(&shape).map_err(|k| span.error(k, "encode result is too large"))?;
        let rows = left.shape().first().copied().unwrap_or(1);
        let columns = crate::array::element_count(left.shape().get(1..).unwrap_or(&[])).map_err(|k| span.error(k, "invalid radix shape"))?;
        let mut result = vec![Element::Number(zero.clone()); count];
        if count != 0 {
            for column in 0..columns {
                for (j, y) in ys.iter().enumerate() {
                    let mut value = y.clone();
                    for row in (0..rows).rev() {
                        let base = &xs[row * columns + column];
                        let digit = base.math_dyad(Math::Magnitude, &value).map_err(error)?;
                        if row != 0 {
                            value = if base.grade_order(&base.unit(0)).is_eq() { zero.clone() } else { value.dyad(Arithmetic::Minus, &digit).and_then(|v| v.dyad(Arithmetic::Divide, base)).map_err(error)? };
                        }
                        result[(row * columns + column) * ys.len() + j] = Element::Number(digit);
                    }
                }
            }
        }
        return Array::from_parts(shape, result, Element::Number(zero)).map_err(|k| span.error(k, "invalid encode result"));
    }
    let xlen = left.shape().last().copied().unwrap_or(1);
    let ylen = right.shape().first().copied().unwrap_or(1);
    if xlen != ylen && xlen != 1 && ylen != 1 { return Err(span.error(ErrorKind::Length, "decode axes do not agree")); }
    let len = if xlen == 1 { ylen } else { xlen };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = right.shape().get(1..).unwrap_or(&[]);
    let shape = [xf, yf].concat();
    let count = generated_len(&shape).map_err(|k| span.error(k, "decode result is too large"))?;
    let columns = crate::array::element_count(yf).map_err(|k| span.error(k, "invalid decode shape"))?;
    let mut data = Vec::with_capacity(count);
    for i in 0..count {
        let (row, column) = (i / columns, i % columns);
        let mut value = zero.clone();
        for k in 0..len {
            let y = &ys[if ylen == 1 { column } else { k * columns + column }];
            value = if k == 0 { y.clone() } else {
                let x = &xs[row * xlen + if xlen == 1 { 0 } else { k }];
                value.dyad(Arithmetic::Times, x).and_then(|v| v.dyad(Arithmetic::Plus, y)).map_err(error)?
            };
        }
        data.push(Element::Number(value));
    }
    Array::from_parts(shape, data, Element::Number(zero)).map_err(|k| span.error(k, "invalid decode result"))
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

fn grade(left: Option<&Array>, right: &Array, down: bool, span: &Context<'_>) -> Result<Array, Error> {
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
    Array::integers(vec![indices.len()], indices.into_iter().map(|i| (i + 1) as i64).collect()).map_err(|k| span.error(k, "invalid grade result"))
}

fn cell_order(left: &Array, right: &Array, span: &Context<'_>) -> Result<std::cmp::Ordering, Error> {
    span.check()?;
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

fn interval_index(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
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
        data.push(generated(lo, true));
    }
    Array::from_parts(shape, data, integer(0)).map_err(|k| span.error(k, "invalid interval index"))
}

fn roll(right: &Array, exact: bool, span: &Context<'_>, fill: bool) -> Result<Array, Error> {
    let fill = fill || right.is_empty();
    let mut rng = rand::rng();
    let mut item = |e| {
        if let Element::Nested(a) = e { return roll(&a, exact, span, fill).map(Element::Nested); }
        if fill { return Ok(generated(0, exact)); }
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "roll needs a nonnegative integer"))?;
        if !exact && n > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "roll bound exceeds exact floating-point integers")); }
        Ok(if n == 0 { float(rng.sample(rand::distr::Open01)) } else { generated(rng.random_range(1..=n), exact) })
    };
    let data = right.elements().map(&mut item).collect::<Result<Vec<_>, _>>()?;
    let prototype = if right.is_empty() { item(right.prototype().clone())? } else { generated(0, exact) };
    Array::from_parts(right.shape().to_vec(), data, prototype).map_err(|k| span.error(k, "invalid roll result"))
}

fn deal(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let count = |a: &Array| {
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "deal needs scalars or singleton vectors")); }
        if !a.is_singleton() { return Err(span.error(ErrorKind::Length, "deal needs one count per argument")); }
        numeric(&a.at(0), span)?.nonnegative_integer().map_err(|k| span.error(k, "deal needs nonnegative integer counts"))
    };
    let (n, total) = (count(left)?, count(right)?);
    if n > total { return Err(span.error(ErrorKind::Domain, "cannot deal more items than the population")); }
    generated_len(&[n]).map_err(|k| span.error(k, "deal exceeds array limits"))?;
    let exact = left.is_exact() && right.is_exact();
    if !exact && total > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "deal population exceeds exact floating-point integers")); }
    let data = rand::seq::index::sample(&mut rand::rng(), total, n).into_iter().map(|i| generated(i + 1, exact)).collect();
    Array::from_parts(vec![n], data, generated(0, exact)).map_err(|k| span.error(k, "invalid deal result"))
}

fn scalar_axes(p: Primitive, left: &Array, right: &Array, spec: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let left_small = left.shape().len() < right.shape().len();
    let (small, large) = if left_small { (left, right) } else { (right, left) };
    let axes = axes(spec, large.shape().len(), span)?;
    if axes.len() != small.shape().len() { return Err(span.error(ErrorKind::Length, "scalar-function axes must match the lower rank")); }
    let agreement = Agreement::with_axes(left.shape(), right.shape(), &axes).map_err(|k| span.error(k, "scalar-function axis lengths differ"))?;
    p.scalar_mapped(Some(left), right, span, false, &agreement)
}

fn reorder(right: &Array, order: &[usize], span: &Context<'_>) -> Result<Array, Error> {
    let mut labels = vec![0; order.len()];
    for (i, &axis) in order.iter().enumerate() { labels[axis] = (i + 1) as i64; }
    transpose(Some(&Array::integers(vec![labels.len()], labels).unwrap()), right, span)
}

fn enclose_axes(right: &Array, axes: &[usize], span: &Context<'_>) -> Result<Array, Error> {
    let mut order: Vec<_> = (0..right.shape().len()).filter(|a| !axes.contains(a)).collect();
    let frame: Vec<_> = order.iter().map(|&a| right.shape()[a]).collect();
    let cell_shape: Vec<_> = axes.iter().map(|&a| right.shape()[a]).collect();
    order.extend(axes);
    let permuted = reorder(right, &order, span)?;
    let data = permuted.cells(axes.len()).map_err(|k| span.error(k, "invalid enclosed cells"))?.into_iter().map(Element::Nested).collect::<Vec<_>>();
    let prototype = if data.is_empty() {
        let len = generated_len(&cell_shape).map_err(|k| span.error(k, "enclosed prototype is too large"))?;
        Element::Nested(
            Array::from_parts(cell_shape, vec![right.prototype().clone(); len], right.prototype().clone())
                .map_err(|k| span.error(k, "invalid enclosed prototype"))?,
        )
    } else { data[0].prototype() };
    Array::from_parts(frame, data, prototype).map_err(|k| span.error(k, "invalid enclosed array"))
}

fn mix_axes(right: &Array, spec: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let mixed = Primitive::Take.call(None, right, span)?;
    let frame = right.shape().len();
    let rank = mixed.shape().len();
    let cell_rank = rank - frame;
    let positions = if spec.is_singleton() {
        let start = match fractional_axis(spec, frame, span)? { Some(a) => a, None => single_axis(spec, span)? };
        if start > frame { return Err(span.error(ErrorKind::Index, "mix axes are outside result rank")); }
        (start..start + cell_rank).collect::<Vec<_>>()
    } else { axes(spec, rank, span)? };
    if positions.len() != cell_rank { return Err(span.error(ErrorKind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(&mixed, &order, span)
}

fn laminate(left: &Array, right: &Array, axis: usize, span: &Context<'_>) -> Result<Array, Error> {
    let shape = if left.is_scalar() { right.shape() } else { left.shape() };
    if !left.is_scalar() && !right.is_scalar() && left.shape() != right.shape() {
        return Err(span.error(ErrorKind::Length, "laminate argument shapes differ"));
    }
    let dimensions = Array::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).unwrap();
    let extend = |a: &Array| {
        let mut extended = shape.to_vec();
        extended.insert(axis, 1);
        let a = if a.is_scalar() { reshape(&dimensions, a, span)? } else { a.clone() };
        a.with_shape(extended).map_err(|k| span.error(k, "invalid laminate shape"))
    };
    catenate(&extend(left)?, &extend(right)?, Some(axis), false, span)
}

fn reshape(dimensions: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
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
    if let Some(values) = right.as_integers() {
        let data = if values.is_empty() { vec![0; len] } else { values.iter().copied().cycle().take(len).collect() };
        return Array::integers(shape, data).map_err(|k| span.error(k, "invalid reshape"));
    }
    let data = if right.is_empty() { vec![right.prototype().clone(); len] } else { right.elements().cycle().take(len).collect() };
    Array::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid reshape"))
}

fn remap(right: &Array, shape: Vec<usize>, source: impl Fn(usize) -> Option<usize>, span: &Context<'_>) -> Result<Array, Error> {
    let len = generated_len(&shape).map_err(|k| span.error(k, "result exceeds array limits"))?;
    if let Some(values) = right.as_floats() {
        return Array::floats(shape, (0..len).map(|i| source(i).map_or(0.0, |j| values[j])).collect()).map_err(|k| span.error(k, "invalid structural result"));
    }
    if let Some(values) = right.as_integers() {
        return Array::integers(shape, (0..len).map(|i| source(i).map_or(0, |j| values[j])).collect()).map_err(|k| span.error(k, "invalid structural result"));
    }
    let data = (0..len).map(|i| source(i).map_or_else(|| right.prototype().clone(), |j| right.at(j))).collect();
    Array::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid structural result"))
}

fn take_drop(take: bool, counts: &Array, right: &Array, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Array, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "take/drop counts must be a scalar or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let old = if right.is_scalar() { vec![1; counts.len()] } else { right.shape().to_vec() };
    let mut shape = old.clone();
    let mut starts = vec![0i128; shape.len()];
    if counts.len() > shape.len() { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    for (i, item) in counts.elements().enumerate() {
        let axis = axes.map_or(i, |a| a[i]);
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

fn replicate(counts: &Array, right: &Array, first: bool, axis: Option<usize>, expand: bool, span: &Context<'_>) -> Result<Array, Error> {
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

fn rotate(counts: Option<&Array>, right: &Array, axis: usize, span: &Context<'_>) -> Result<Array, Error> {
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

fn catenate(left: &Array, right: &Array, axis: Option<usize>, first: bool, span: &Context<'_>) -> Result<Array, Error> {
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

fn transpose(axes: Option<&Array>, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
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

fn split(right: &Array, axis: Option<usize>, span: &Context<'_>) -> Result<Array, Error> {
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

fn partition(left: &Array, right: &Array, axis: Option<usize>, runs: bool, span: &Context<'_>) -> Result<Array, Error> {
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

fn squad(left: &Array, right: &Array, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Array, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "squad indices must be a scalar or vector")); }
    if left.len() > right.shape().len() || axes.is_some_and(|a| a.len() != left.len()) {
        return Err(span.error(ErrorKind::Length, "squad needs one index item per selected axis"));
    }
    let mut parts = vec![None; right.shape().len()];
    for (i, item) in left.elements().enumerate() {
        let coords = item.as_array();
        for n in coords.elements() { numeric(&n, span)?; }
        let part = parts.get_mut(axes.map_or(i, |a| a[i])).ok_or_else(|| span.error(ErrorKind::Rank, "squad axis is outside array rank"))?;
        *part = Some(coords);
    }
    select(right, &parts, span)
}

fn coordinate_offset(coords: &Array, right: &Array, span: &Context<'_>) -> Result<usize, Error> {
    if coords.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a coordinate must be a scalar or vector")); }
    if coords.len() != right.shape().len() { return Err(span.error(ErrorKind::Length, "a coordinate needs one index per axis")); }
    let mut offset = 0;
    for (n, &size) in coords.elements().zip(right.shape()) { offset = offset * size + index(numeric(&n, span)?, size, span)?; }
    Ok(offset)
}

fn pick(left: &Array, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a pick path must be a scalar or vector")); }
    let mut result = right.clone();
    for item in left.elements() {
        let offset = coordinate_offset(&item.as_array(), &result, span)?;
        result = result.at(offset).as_array();
    }
    Ok(result)
}

fn index(n: &Number, limit: usize, span: &Context<'_>) -> Result<usize, Error> {
    let n = n.integer().map_err(|k| span.error(k, "index must be a positive integer"))?;
    if n <= 0 || n as usize > limit { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
    Ok(n as usize - 1)
}

pub(crate) struct Selection { pub shape: Vec<usize>, pub paths: Vec<Vec<usize>> }

impl Selection {
    pub(crate) fn read(&self, array: &Array, span: &Context<'_>) -> Result<Array, Error> {
        let data = self
            .paths
            .iter()
            .map(|path| {
                let mut item = Element::Nested(array.clone());
                for &i in path { item = item.as_array().at(i); }
                item
            })
            .collect();
        Array::from_parts(self.shape.clone(), data, array.prototype().clone()).map_err(|k| span.error(k, "invalid selection"))
    }

    pub(crate) fn write(&self, array: &Array, values: &Array, span: &Context<'_>) -> Result<Array, Error> {
        if !values.is_singleton() && values.shape() != self.shape { return Err(span.error(ErrorKind::Length, "replacement shape does not match selection")); }
        fn replace(array: &Array, updates: &[(&[usize], Element)], span: &Context<'_>) -> Result<Array, Error> {
            let mut data: Vec<_> = array.elements().collect();
            let mut groups: HashMap<usize, Vec<(&[usize], Element)>> = HashMap::new();
            for (path, value) in updates { groups.entry(path[0]).or_default().push((&path[1..], value.clone())); }
            for (i, edits) in groups {
                if i >= data.len() { return Err(span.error(ErrorKind::Index, "replacement changed a selected path")); }
                let mut item = data[i].clone();
                let start = if let Some(last) = edits.iter().rposition(|(p, _)| p.is_empty()) {
                    item = edits[last].1.clone();
                    last + 1
                } else { 0 };
                if start < edits.len() { item = Element::Nested(replace(&item.as_array(), &edits[start..], span)?); }
                data[i] = item;
            }
            Array::from_parts(array.shape().to_vec(), data, array.prototype().clone()).map_err(|k| span.error(k, "invalid amended array"))
        }
        let updates: Vec<_> = self.paths.iter().enumerate().map(|(i, p)| (p.as_slice(), selected(values, i))).collect();
        replace(array, &updates, span)
    }
}

pub(crate) fn choose(right: &Array, indices: &Array, span: &Context<'_>) -> Result<Selection, Error> {
    let mut paths = Vec::with_capacity(indices.len());
    for item in indices.elements() {
        let coordinates = item.as_array();
        let reach = coordinates.elements().any(|e| matches!(e, Element::Nested(_)));
        let steps = if reach { coordinates.elements().map(|e| e.as_array()).collect() } else { vec![coordinates] };
        let mut current = right.clone();
        let mut path = Vec::new();
        for coords in steps {
            let offset = coordinate_offset(&coords, &current, span)?;
            path.push(offset);
            current = current.at(offset).as_array();
        }
        paths.push(path);
    }
    Ok(Selection { shape: indices.shape().to_vec(), paths })
}

pub(crate) fn at_indices(right: &Array, indices: &Array, span: &Context<'_>) -> Result<Selection, Error> {
    if matches!(indices.elements().next().unwrap_or_else(|| indices.prototype().clone()), Element::Nested(_)) { return choose(right, indices, span); }
    if indices.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "at needs scalar/vector major-cell indices")); }
    if right.is_scalar() { return Err(span.error(ErrorKind::Length, "a scalar has no major-cell axis")); }
    let shape = [indices.shape(), &right.shape()[1..]].concat();
    generated_len(&shape).map_err(|k| span.error(k, "selection is too large"))?;
    let size = crate::array::element_count(&right.shape()[1..]).map_err(|k| span.error(k, "invalid major cell"))?;
    let mut paths = Vec::new();
    for n in indices.elements() {
        let row = index(numeric(&n, span)?, right.shape()[0], span)?;
        paths.extend((0..size).map(|j| vec![row * size + j]));
    }
    Ok(Selection { shape, paths })
}

pub(crate) fn select(right: &Array, parts: &[Option<Array>], span: &Context<'_>) -> Result<Array, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    selection(right, parts, span)?.read(right, span)
}

pub(crate) fn selection(right: &Array, parts: &[Option<Array>], span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() { return Ok(Selection { shape: right.shape().to_vec(), paths: (0..right.len()).map(|i| vec![i]).collect() }); }
    if let [Some(indices)] = parts {
        if matches!(indices.elements().next().unwrap_or_else(|| indices.prototype().clone()), Element::Nested(_)) { return choose(right, indices, span); }
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
    let count = generated_len(&shape).map_err(|k| span.error(k, "selection is too large"))?;
    let paths = (0..count)
        .map(|mut flat| {
            let (mut source, mut stride) = (0, 1);
            for axis in (0..parts.len()).rev() {
                let len = indices[axis].as_ref().map_or(right.shape()[axis], Vec::len);
                let coord = flat % len;
                flat /= len;
                source += indices[axis].as_ref().map_or(coord, |v| v[coord]) * stride;
                stride *= right.shape()[axis];
            }
            vec![source]
        })
        .collect();
    Ok(Selection { shape, paths })
}
