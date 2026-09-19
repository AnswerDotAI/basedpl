use crate::{
    agreement::{Agreement, Mapping},
    array::{generated_len, Axis, Frame, MAX_GENERATED_ELEMENTS},
    execution::Context,
    number::{Arithmetic, Math},
    Error, ErrorKind, Number, Span, Value,
};
use rand::RngExt;
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
    History,
    PairInverse,
    Under,
    Differentiate,
    Agenda,
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
            Outer => "⌝",
            Key => "⌸",
            Power => "⍣",
            History => "⍣\\",
            PairInverse => "⇄",
            Under => "⌾",
            Differentiate => "∂",
            Agenda => "◶",
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
    Mix,
    Take,
    Drop,
    Replicate(bool),
    Expand(bool),
    CatenateFirst,
    Reverse(bool),
    Transpose,
    Windows,
    Prime,
    Factor,
    Polynomial,
}

pub(crate) fn numeric<'a>(e: &'a Value, span: &Span) -> Result<&'a Number, Error> {
    match e { Value::Number(n) => Ok(n), _ => Err(span.error(ErrorKind::Domain, "expected numeric elements")) }
}
fn float(n: f64) -> Value { Value::Number(Number::try_from(n).expect("finite generated number")) }
pub(crate) fn integer(n: i64) -> Value { Value::Number(Number::from_integer(n)) }
fn generated(n: usize, exact: bool) -> Value {
    if !exact { return float(n as f64); }
    match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(Number::try_from(num_rational::BigRational::from_integer(n.into())).unwrap()) }
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

fn windows(sizes: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if sizes.shape().len() > 1 || sizes.len() > right.shape().len() {
        return Err(span.error(ErrorKind::Rank, "window sizes must be a scalar or vector within the argument rank"));
    }
    if sizes.is_empty() { return Ok(right.clone()); }
    let sizes = sizes
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "window sizes must be nonnegative integers")))
        .collect::<Result<Vec<_>, _>>()?;
    let frame = sizes
        .iter()
        .zip(right.shape())
        .map(|(&w, &n)| n.checked_add(1).map(|n| n.saturating_sub(w)).ok_or_else(|| span.error(ErrorKind::Limit, "window frame is too large")))
        .collect::<Result<Vec<_>, _>>()?;
    let cell = [&sizes, &right.shape()[sizes.len()..]].concat();
    let shape = [frame.as_slice(), cell.as_slice()].concat();
    let len = generated_len(&shape).map_err(|k| span.error(k, "windows exceed array limits"))?;
    if len == 0 { return Value::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid empty windows")); }
    let width = generated_len(&cell).map_err(|k| span.error(k, "window is too large"))?;
    let mut data = Vec::with_capacity(len);
    for i in 0..len / width {
        span.check()?;
        let (mut rest, mut starts) = (i, vec![0; frame.len()]);
        for a in (0..frame.len()).rev() {
            starts[a] = rest % frame[a];
            rest /= frame[a];
        }
        for mut j in 0..width {
            let (mut offset, mut stride) = (0, 1);
            for a in (0..cell.len()).rev() {
                offset += (j % cell[a] + starts.get(a).copied().unwrap_or(0)) * stride;
                j /= cell[a];
                stride *= right.shape()[a];
            }
            data.push(right.at(offset));
        }
    }
    Value::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid windows"))
}

pub(crate) fn axis_value(axis: usize) -> Value { Value::scalar(Number::from_integer((axis + 1) as i64)).unwrap() }
pub(crate) fn single_axis(axis: &Value, span: &Span) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "one axis is required")); }
    numeric(&axis.at(0), span)?
        .nonnegative_integer()
        .map_err(|k| span.error(k, "axis must be integral"))?
        .checked_sub(1)
        .ok_or_else(|| span.error(ErrorKind::Domain, "axes start at one"))
}

fn axes(axis: &Value, rank: usize, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be scalar or vector")); }
    let mut result = Vec::new();
    for e in axis.elements() {
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "axis must be integral"))?;
        if n == 0 || n > rank || result.contains(&(n - 1)) { return Err(span.error(ErrorKind::Domain, "axes must be distinct and within the array rank")); }
        result.push(n - 1);
    }
    Ok(result)
}

fn fractional_axis(axis: &Value, rank: usize, span: &Context<'_>) -> Result<Option<usize>, Error> {
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

fn float_apply(op: Primitive, left: Option<&[f64]>, right: &[f64], agreement: &Agreement, span: &Context<'_>) -> Result<Value, Error> {
    use crate::number::float_equal;
    use Arithmetic::*;
    use Comparison::*;
    if matches!(op, Primitive::Arithmetic(Divide))
        && (0..agreement.len).any(|i| right[agreement.right.index(i)] == 0.0 && left.is_none_or(|x| x[agreement.left.index(i)] != 0.0))
    { return Err(span.error(ErrorKind::Domain, "division by zero")); }
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
            return Value::integers(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid comparison result"));
        }
        _ => unreachable!(),
    };
    Value::floats(agreement.shape.clone(), data).map_err(|k| span.error(k, "undefined real result"))
}

/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    pub(crate) fn glyph(self) -> &'static str {
        use Arithmetic::*;
        use Comparison::*;
        use Math::*;
        match self {
            Self::Arithmetic(p) => match p {
                Plus => "+",
                Minus => "-",
                Times => "×",
                Divide => "÷",
            },
            Self::Math(p) => match p {
                Magnitude => "|",
                Floor => "⌊",
                Ceiling => "⌈",
                Power => "*",
                Log => "⍟",
                Circle => "○",
                Pi => "π",
                Root => "√",
                Factorial => "!",
                Gcd => "∨",
                Lcm => "∧",
                Nand => "⍲",
                Nor => "⍱",
                Not => "~",
            },
            Self::Compare(p) => match p {
                Equal => "=",
                NotEqual => "≠",
                Less => "<",
                LessEqual => "≤",
                Greater => ">",
                GreaterEqual => "≥",
            },
            Self::Random => "?",
            Self::Identity(true) => "⊣",
            Self::Identity(false) => "⊢",
            Self::Iota => "⍳",
            Self::Depth => "≡",
            Self::Where => "⍸",
            Self::Member => "∊",
            Self::Union => "∪",
            Self::Intersection => "∩",
            Self::Find => "⍷",
            Self::Grade(false) => "⍋",
            Self::Grade(true) => "⍒",
            Self::Index => "⌷",
            Self::Encode => "⊤",
            Self::Decode => "⊥",
            Self::MatrixDivide => "⌹",
            Self::Execute => "⍎",
            Self::Format => "⍕",
            Self::Shape => "⍴",
            Self::Tally => "≢",
            Self::Ravel => ",",
            Self::Enclose => "⊂",
            Self::Nest => "⊆",
            Self::Mix => "⊃",
            Self::Take => "↑",
            Self::Drop => "↓",
            Self::Replicate(false) => "/",
            Self::Replicate(true) => "⌿",
            Self::Expand(false) => "\\",
            Self::Expand(true) => "⍀",
            Self::CatenateFirst => "⍪",
            Self::Reverse(false) => "⌽",
            Self::Reverse(true) => "⊖",
            Self::Transpose => "⍉",
            Self::Windows => "↕",
            Self::Prime => "ℙ",
            Self::Factor => "Ⓠ",
            Self::Polynomial => "Ⓟ",
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
            'π' => Self::Math(Math::Pi),
            '√' => Self::Math(Math::Root),
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
            '⊃' => Self::Mix,
            '↑' => Self::Take,
            '↓' => Self::Drop,
            '⍪' => Self::CatenateFirst,
            '⌽' => Self::Reverse(false),
            '⊖' => Self::Reverse(true),
            '⍉' => Self::Transpose,
            '↕' => Self::Windows,
            'ℙ' => Self::Prime,
            'Ⓠ' => Self::Factor,
            'Ⓟ' => Self::Polynomial,
            _ => return None,
        })
    }
    pub(crate) fn call(self, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> { self.call_axis(left, right, None, span) }
    pub(crate) fn call_axes(self, left: Option<&Value>, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
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
            (Self::Mix, None) => mix_axes(right, spec, span),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, Some(&axes(spec, right.shape().len(), span)?), span),
            (Self::Index, Some(x)) => squad(x, right, Some(&axes(spec, right.shape().len(), span)?), span),
            (Self::CatenateFirst, None) => Err(span.error(ErrorKind::Syntax, "table does not accept an axis")),
            _ => self.call_axis(left, right, Some(single_axis(spec, span)?), span),
        }
    }
    pub(crate) fn call_axis(self, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
        span.check()?;
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
                    | Self::Mix
                    | Self::Index
                    | Self::Enclose
                    | Self::Nest
            )
        { return Err(span.error(ErrorKind::Syntax, "axis is not supported by this primitive")); }
        match self {
            Self::Identity(first) => return Ok(if first { left.unwrap_or(right) } else { right }.clone()),
            Self::Math(Math::Nand | Math::Nor) if left.is_none() => {
                let op = if matches!(self, Self::Math(Math::Nand)) { Arithmetic::Times } else { Arithmetic::Plus };
                return Self::Arithmetic(op).call(Some(right), right, span);
            }
            Self::Prime | Self::Factor => return crate::number_theory::call(matches!(self, Self::Factor), left, right, span),
            Self::Polynomial => return crate::polynomial::call(left, right, span),
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
                return match left {
                    Some(x) => radix(x, right, matches!(self, Self::Encode), span),
                    None if matches!(self, Self::Encode) => binary_encode(right, span),
                    None => radix(&integer(2), right, false, span),
                };
            }
            Self::Math(Math::Gcd | Math::Lcm) if left.is_none() => return complex_parts(right, matches!(self, Self::Math(Math::Lcm)), span),
            Self::Compare(Comparison::Equal) if left.is_none() => return self_classify(right, span),
            Self::Compare(Comparison::LessEqual | Comparison::GreaterEqual) if left.is_none() => {
                let op = if matches!(self, Self::Compare(Comparison::LessEqual)) { Arithmetic::Minus } else { Arithmetic::Plus };
                return Self::Arithmetic(op).call(Some(right), &integer(1), span);
            }
            Self::Index => {
                return match left { Some(x) => squad(x, right, axis.map(|a| vec![a]).as_deref(), span), None => Ok(right.clone()) }
            }
            Self::Math(Math::Not) if left.is_some() => return without(left.unwrap(), right, span),
            Self::Depth => {
                return match left { Some(x) => Ok(integer(i64::from(array_match(x, right, span)?))), None => Ok(integer(depth(right) as i64)) }
            }
            Self::Tally if left.is_some() => return Ok(integer(i64::from(!array_match(left.unwrap(), right, span)?))),
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
            Self::Windows => return windows(left.ok_or_else(|| span.error(ErrorKind::Syntax, "windows needs sizes on the left"))?, right, span),
            Self::Ravel | Self::CatenateFirst if left.is_some() => return catenate(left.unwrap(), right, axis, matches!(self, Self::CatenateFirst), span),
            Self::CatenateFirst => {
                if axis.is_some() { return Err(span.error(ErrorKind::Syntax, "table does not take an axis")); }
                let rows = right.shape().first().copied().unwrap_or(1);
                let columns = crate::array::element_count(right.shape().get(1..).unwrap_or(&[])).map_err(|k| span.error(k, "invalid table shape"))?;
                return right.with_shape(vec![rows, columns]).map_err(|k| span.error(k, "invalid table"));
            }
            Self::Shape if left.is_some() => return reshape(left.unwrap(), right, span),
            Self::Mix => {
                if let Some(x) = left { return pick(x, right, false, span); }
                if let Some(axis) = axis { return mix_axes(right, &axis_value(axis), span); }
                let cells: Vec<_> = right.elements().collect();
                return Value::assemble(right.shape(), &cells, &right.prototype()).map_err(|k| span.error(k, "cannot assemble cells"));
            }
            Self::Enclose | Self::Nest => {
                if let Some(x) = left { return partition(x, right, axis, matches!(self, Self::Nest), span); }
                if let Some(axis) = axis {
                    if matches!(self, Self::Nest) { return Err(span.error(ErrorKind::Syntax, "nest does not take an axis")); }
                    return self.call_axes(None, right, &axis_value(axis), span);
                }
                if matches!(self, Self::Nest) && right.elements().chain(std::iter::once(right.prototype().clone())).any(|e| matches!(e, Value::Array(_))) {
                    return Ok(right.clone());
                }
                return Value::new(vec![], vec![right.clone()]).map_err(|k| span.error(k, "invalid enclosure"));
            }
            Self::Take | Self::Drop => {
                if left.is_none() && matches!(self, Self::Take) { return pick(&integer(1), right, false, span); }
                if left.is_none() { return split(right, axis, span); }
                return take_drop(matches!(self, Self::Take), left.unwrap(), right, axis.map(|a| vec![a]).as_deref(), span);
            }
            Self::Shape | Self::Tally | Self::Ravel => {
                return match self {
                    Self::Shape => Value::from_parts(vec![right.shape().len()], right.shape().iter().map(|&n| generated(n, true)).collect(), integer(0))
                        .map_err(|k| span.error(k, "invalid shape")),
                    Self::Tally => Ok(generated(right.shape().first().copied().unwrap_or(1), true)),
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

    fn scalar_apply(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        if matches!(self, Self::Random) && !fill { return roll(right, right.is_exact(), span, false); }
        if right.is_atom() && left.is_none_or(Value::is_atom) { return self.scalar_item(left, right, span, fill); }
        let agreement = Agreement::new(left.map_or(&[], Value::shape), right.shape()).map_err(|k| span.error(k, "array shapes do not agree"))?;
        self.scalar_mapped(left, right, span, fill, &agreement)
    }
    fn scalar_mapped(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool, agreement: &Agreement) -> Result<Value, Error> {
        if agreement.len == 0 {
            let prototype = self.scalar_item(left.map(Value::prototype).as_ref(), &right.prototype(), span, true)?;
            return Value::empty(agreement.shape.clone(), prototype).map_err(|k| span.error(k, "invalid empty result"));
        }
        if matches!(self, Self::Arithmetic(_) | Self::Compare(_)) && right.as_floats().is_some() && left.is_none_or(|a| a.as_floats().is_some()) {
            return float_apply(self, left.map(|a| a.as_floats().unwrap()), right.as_floats().unwrap(), agreement, span);
        }
        if let (Self::Compare(op), Some(x), Some(y)) = (self, left.and_then(Value::as_integers), right.as_integers()) {
            let data = (0..agreement.len).map(|i| i64::from(op.ordered(x[agreement.left.index(i)].cmp(&y[agreement.right.index(i)])))).collect();
            return Value::integers(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid comparison result"));
        }
        if let Self::Arithmetic(op) = self {
            if let Some(y) = right.as_integers() {
                if left.is_none_or(|a| a.as_integers().is_some()) {
                    let x = left.map(|a| a.as_integers().unwrap());
                    let data = (0..agreement.len)
                        .map(|i| Number::checked_integer(op, x.map(|v| v[agreement.left.index(i)]), y[agreement.right.index(i)]))
                        .collect::<Option<Vec<_>>>();
                    if let Some(data) = data { return Value::integers(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid integer result")); }
                }
            }
        }
        let data = (0..agreement.len)
            .map(|i| self.scalar_item(left.map(|a| a.at(agreement.left.index(i))).as_ref(), &right.at(agreement.right.index(i)), span, fill))
            .collect::<Result<_, _>>()?;
        Value::new(agreement.shape.clone(), data).map_err(|k| span.error(k, "invalid scalar result"))
    }

    fn scalar_item(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        span.check()?;
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Value::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Value::Array(_)) || matches!(left, Some(Value::Array(_))) { return self.scalar_apply(left, right, span, fill); }
        if let (Self::Arithmetic(op @ (Arithmetic::Plus | Arithmetic::Minus)), Some(left)) = (self, left) {
            if matches!(right, Value::Character(_)) || matches!(left, Value::Character(_)) {
                return character_arithmetic(op, left, right, fill).map_err(|m| span.error(ErrorKind::Domain, m));
            }
        }
        if fill {
            return Ok(match (self, left, right) {
                (Self::Compare(_) | Self::Math(Math::Not), _, _) | (Self::Math(Math::Nand | Math::Nor), Some(_), _) => integer(0),
                (Self::Random, _, Value::Number(y)) => Value::Number(y.unit(0)),
                (Self::Math(Math::Circle | Math::Pi | Math::Log), _, _) | (Self::Math(Math::Power), None, _) => float(0.0),
                (Self::Math(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Math(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                (Self::Arithmetic(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Arithmetic(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                _ => float(0.0),
            });
        }
        match self {
            Self::Math(op) => {
                let y = numeric(right, span)?;
                let n = match left { Some(x) => numeric(x, span)?.math_dyad(op, y), None => y.math_monad(op) };
                n.map(Value::Number).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            Self::Arithmetic(op) => {
                let y = numeric(right, span)?;
                let n = match left { Some(x) => numeric(x, span)?.dyad(op, y), None => y.monad(op) };
                n.map(Value::Number).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            Self::Compare(op) => {
                use Comparison::*;
                let x = left.unwrap();
                let result = match op {
                    Equal | NotEqual => {
                        let equal = match (x, right) {
                            (Value::Number(x), Value::Number(y)) => x.equal(y),
                            (Value::Character(x), Value::Character(y)) => Ok(x == y),
                            (Value::Function(x), Value::Function(y)) => Ok(x == y),
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

fn character_arithmetic(op: Arithmetic, left: &Value, right: &Value, fill: bool) -> Result<Value, &'static str> {
    use Value::{Character, Number};
    let shift = |c: char, n: &crate::Number, subtract: bool| {
        let offset = n.integer().map_err(|_| "character offset must be a finite real integer")? as i128;
        let value = if subtract { c as i128 - offset } else { c as i128 + offset };
        let result = u32::try_from(value).ok().and_then(char::from_u32).ok_or("character result is not a Unicode scalar value")?;
        Ok(Character(if fill { ' ' } else { result }))
    };
    match (op, left, right) {
        (Arithmetic::Plus, Character(c), Number(n)) | (Arithmetic::Plus, Number(n), Character(c)) => shift(*c, n, false),
        (Arithmetic::Minus, Character(c), Number(n)) => shift(*c, n, true),
        (Arithmetic::Minus, Character(x), Character(y)) => Ok(integer(if fill { 0 } else { *x as i64 - *y as i64 })),
        _ => Err("unsupported character arithmetic"),
    }
}

pub(crate) fn array_match(left: &Value, right: &Value, span: &Context<'_>) -> Result<bool, Error> {
    if left.is_atom() || right.is_atom() { return element_match(left, right, span); }
    span.check()?;
    if left.shape() != right.shape() { return Ok(false); }
    if left.is_empty() { return element_match(&left.prototype(), &right.prototype(), span); }
    for (x, y) in left.elements().zip(right.elements()) { if !element_match(&x, &y, span)? { return Ok(false); } }
    Ok(true)
}

fn element_match(left: &Value, right: &Value, span: &Context<'_>) -> Result<bool, Error> {
    span.check()?;
    match (left, right) {
        (Value::Number(x), Value::Number(y)) => x.equal(y).map_err(|m| span.error(ErrorKind::Domain, m)),
        (Value::Character(x), Value::Character(y)) => Ok(x == y),
        (Value::Function(x), Value::Function(y)) => Ok(x == y),
        (x @ Value::Array(_), y @ Value::Array(_)) => array_match(x, y, span),
        _ => Ok(false),
    }
}

fn depth(right: &Value) -> isize {
    if right.is_atom() { return 0; }
    let items = right.elements().chain(right.is_empty().then(|| right.prototype().clone()));
    if items.clone().all(|e| !matches!(e, Value::Array(_))) { return 1; }
    let ds: Vec<_> = items
        .map(|e| match e { a @ Value::Array(_) => depth(&a), _ => 0 })
        .collect();
    let n = 1 + ds.iter().map(|d| d.abs()).max().unwrap();
    if ds.iter().all(|&d| d >= 0 && d == ds[0]) { n } else { -n }
}

fn without(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "without needs a scalar or vector left argument")); }
    let mut data = Vec::new();
    for x in left.elements() { if !contains(right, &x, span)? { data.push(x); } }
    Value::from_parts(vec![data.len()], data, left.prototype().clone()).map_err(|k| span.error(k, "invalid without result"))
}

fn contains(array: &Value, element: &Value, span: &Context<'_>) -> Result<bool, Error> {
    for item in array.elements() { if element_match(&item, element, span)? { return Ok(true); } }
    Ok(false)
}

fn membership(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let data = left.elements().map(|e| contains(right, &e, span).map(|v| integer(i64::from(v)))).collect::<Result<_, _>>()?;
    let frame = if left.shape().is_empty() { Frame::Direct } else { Frame::Array(left.shape().to_vec()) };
    frame.collect(data, integer(0)).map_err(|k| span.error(k, "invalid membership result"))
}

fn enlist(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn append(array: &Value, data: &mut Vec<Value>, span: &Context<'_>) -> Result<(), Error> {
        for item in array.elements() {
            if let a @ Value::Array(_) = item { append(&a, data, span)?; } else {
                if data.len() == MAX_GENERATED_ELEMENTS { return Err(span.error(ErrorKind::Limit, "enlist exceeds array limits")); }
                data.push(item);
            }
        }
        Ok(())
    }
    let mut data = Vec::new();
    append(right, &mut data, span)?;
    let mut prototype = right.prototype();
    while let a @ Value::Array(_) = prototype { prototype = a.prototype(); }
    Value::from_parts(vec![data.len()], data, prototype.clone()).map_err(|k| span.error(k, "invalid enlist result"))
}

fn unique(right: &Value, span: &Context<'_>) -> Result<Value, Error> { replicate(&unique_mask(right, span)?, right, true, None, false, span) }

fn union(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "union needs scalars or vectors")); }
    catenate(left, &without(right, left, span)?, None, false, span)
}

fn intersection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "intersection needs scalars or vectors")); }
    replicate(&membership(left, right, span)?, left, true, None, false, span)
}

fn find(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
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
    Frame::of(right).collect(data, integer(0)).map_err(|k| span.error(k, "invalid find result"))
}

fn self_classify(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len().saturating_sub(1);
    let cells = right.cells(rank).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid major cells"))?;
    let distinct = unique(right, span)?;
    let classes = distinct.cells(rank).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid classes"))?;
    let shape = vec![classes.len(), cells.len()];
    let mut data = Vec::with_capacity(generated_len(&shape).map_err(|k| span.error(k, "classification is too large"))?);
    for class in &classes { for cell in &cells { data.push(integer(i64::from(array_match(class, cell, span)?))); } }
    Value::from_parts(shape, data, integer(0)).map_err(|k| span.error(k, "invalid classification"))
}

fn complex_parts(right: &Value, polar: bool, span: &Context<'_>) -> Result<Value, Error> {
    let shape = [right.shape(), &[2]].concat();
    let mut data = Vec::with_capacity(generated_len(&shape).map_err(|k| span.error(k, "decomposition is too large"))?);
    for item in right.elements() {
        span.check()?;
        data.extend(numeric(&item, span)?.parts(polar).map_err(|m| span.error(ErrorKind::Domain, m))?.map(Value::Number));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.unit(0));
    Value::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid decomposition"))
}

fn unique_mask(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let cells = right.cells(right.shape().len().saturating_sub(1)).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid major cells"))?;
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
    Value::from_parts(vec![data.len()], data, integer(0)).map_err(|k| span.error(k, "invalid unique mask"))
}

fn coordinates(shape: &[usize], mut flat: usize, exact: bool) -> Value {
    let mut data = vec![generated(0, exact); shape.len()];
    for axis in (0..shape.len()).rev() {
        data[axis] = generated(flat % shape[axis] + 1, exact);
        flat /= shape[axis];
    }
    Value::from_parts(vec![data.len()], data, generated(0, exact)).unwrap()
}

fn iota(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "iota needs a scalar or vector shape")); }
    let shape = right
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid iota dimension")))
        .collect::<Result<Vec<_>, _>>()?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "iota exceeds array limits"))?;
    let exact = right.is_exact();
    if right.is_singleton() {
        return if exact { Value::integers(shape, (1..=len).map(|i| i as i64).collect()) } else { Value::floats(shape, (1..=len).map(|i| i as f64).collect()) }
            .map_err(|k| span.error(k, "invalid iota"));
    }
    let zero = generated(0, exact);
    let prototype = Value::from_parts(vec![shape.len()], vec![zero.clone(); shape.len()], zero).unwrap();
    let data = (0..len).map(|i| coordinates(&shape, i, exact)).collect();
    Value::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid coordinate array"))
}

fn where_indices(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut data = Vec::new();
    for (i, e) in right.elements().enumerate() {
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "where needs nonnegative integer counts"))?;
        if n > MAX_GENERATED_ELEMENTS - data.len() { return Err(span.error(ErrorKind::Limit, "where exceeds array limits")); }
        let index = if right.shape().len() == 1 { generated(i + 1, true) } else { coordinates(right.shape(), i, true) };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype = if right.shape().len() == 1 { integer(0) } else { Value::integers(vec![right.shape().len()], vec![0; right.shape().len()]).unwrap() };
    Value::from_parts(vec![data.len()], data, prototype).map_err(|k| span.error(k, "invalid where result"))
}

struct SearchCells { left: Vec<Value>, right: Vec<Value>, frame: Frame }

fn search_cells(left: &Value, right: &Value, span: &Context<'_>) -> Result<SearchCells, Error> {
    if left.is_scalar() { return Err(span.error(ErrorKind::Rank, "search needs a non-scalar left argument")); }
    let rank = left.shape().len() - 1;
    let split = right.shape().len().checked_sub(rank).ok_or_else(|| span.error(ErrorKind::Rank, "right argument has insufficient rank"))?;
    if left.shape()[1..] != right.shape()[split..] { return Err(span.error(ErrorKind::Length, "search cell shapes do not agree")); }
    Ok(SearchCells {
        left: left.cells(rank).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid search cells"))?,
        right: right.cells(rank).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid search cells"))?,
        frame: if split == 0 { Frame::Direct } else { Frame::Array(right.shape()[..split].to_vec()) },
    })
}

fn index_of(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let SearchCells { left: haystack, right: needles, frame } = search_cells(left, right, span)?;
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
    frame.collect(data, integer(0)).map_err(|k| span.error(k, "invalid index-of result"))
}

fn format_number(n: &Number, precision: isize, span: &Context<'_>) -> Result<String, Error> {
    use num_traits::Signed;
    if n.as_complex().is_some() { return Err(span.error(ErrorKind::Domain, "specified format requires real numbers")); }
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
        let y = n.to_complex().map_err(|m| span.error(ErrorKind::Domain, m))?.re;
        let exponent = if y == 0.0 { 0 } else { y.abs().log10().floor() as i32 };
        let places = if precision >= 0 { digits as i32 } else { digits as i32 - 1 - exponent };
        let scale = 10_f64.powi(places);
        let scaled = y * scale;
        let y = if scale.is_finite() && scale != 0.0 && scaled.abs() < 1e16 { scaled.round() / scale } else { y };
        if precision >= 0 { if y == 0.0 && y.is_sign_negative() { format!(" {:.*}", digits, 0.0) } else { format!("{y:.digits$}") } } else {
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

fn format_array(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
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
    numeric(&right.prototype(), span)?;
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
        let s = if len > width { "*".repeat(width) } else if specs[i % columns].1 < 0 {
            let leading = usize::from(specs[i % columns].0 == 0);
            format!("{}{s}{}", " ".repeat(leading), " ".repeat(width - len - leading))
        } else { format!("{}{s}", " ".repeat(width - len)) };
        data.extend(s.chars().map(Value::Character));
    }
    Value::from_parts(shape, data, Value::Character(' ')).map_err(|k| span.error(k, "invalid formatted result"))
}

pub(crate) fn lambert_w(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn map(a: &Value, fill: bool, span: &Context<'_>) -> Result<Value, Error> {
        let item = |e: Value, fill| {
            span.check()?;
            match e {
                a @ Value::Array(_) => map(&a, fill, span),
                _ if fill => Ok(float(0.0)),
                _ => numeric(&e, span)?.lambert_w().map(Value::Number).map_err(|message| span.error(ErrorKind::Domain, message)),
            }
        };
        if a.is_atom() { return item(a.clone(), fill); }
        let result = if a.is_empty() { Value::empty(a.shape().to_vec(), item(a.prototype().clone(), true)?) } else { Value::new(a.shape().to_vec(), a.elements().map(|e| item(e, fill)).collect::<Result<_, _>>()?) };
        result.map_err(|k| span.error(k, "invalid Lambert W result"))
    }
    map(right, false, span)
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Context<'_>) -> Result<Value, Error> {
    use crate::number::Arithmetic::*;
    use Primitive::*;
    if let Some(axis) = axis {
        if axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be scalar or vector")); }
        if bound.is_none() {
            return match p {
                Reverse(_) => p.call_axes(None, right, axis, span),
                Enclose => mix_axes(right, axis, span),
                Drop => mix_axes(right, &axis_value(single_axis(axis, span)?), span),
                Mix => {
                    let axes = if axis.is_singleton() {
                        vec![match fractional_axis(axis, right.shape().len().saturating_sub(1), span)? {
                            Some(a) => a,
                            None => single_axis(axis, span)?,
                        }]
                    } else { axes(axis, right.shape().len(), span)? };
                    if axes.iter().any(|&a| a >= right.shape().len()) { return Err(span.error(ErrorKind::Index, "inverse mix axis is outside result rank")); }
                    enclose_axes(right, &axes, span)
                }
                _ => Err(span.error(ErrorKind::Domain, "this axis-qualified primitive has no known inverse")),
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_)) {
            return Err(span.error(ErrorKind::Domain, "this axis-qualified primitive has no known inverse"));
        }
    }
    let call = |p: Primitive, x: Option<&Value>, y: &Value| match axis { Some(axis) => p.call_axes(x, y, axis, span), None => p.call(x, y, span) };
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide)) && a.elements().any(|e| matches!(e, Value::Number(n) if n.equal(&n.unit(0)).unwrap_or(false))) {
            return Err(span.error(ErrorKind::Domain, "zero multiplier/divisor has no inverse"));
        }
        return match p {
            Arithmetic(Plus) => call(Arithmetic(Minus), Some(right), a),
            Arithmetic(Minus) if first => call(p, Some(a), right),
            Arithmetic(Minus) => call(Arithmetic(Plus), Some(right), a),
            Arithmetic(Times) => call(Arithmetic(Divide), Some(right), a),
            Arithmetic(Divide) if first => call(p, Some(a), right),
            Arithmetic(Divide) => call(Arithmetic(Times), Some(right), a),
            Compare(Comparison::NotEqual) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Math(crate::number::Math::Power) if first => call(Math(crate::number::Math::Log), Some(a), right),
            Math(crate::number::Math::Power) => call(p, Some(right), &Arithmetic(Divide).call(None, a, span)?),
            Math(crate::number::Math::Root) if first => call(Math(crate::number::Math::Power), Some(right), a),
            Math(crate::number::Math::Root) => call(Math(crate::number::Math::Log), Some(right), a),
            Math(crate::number::Math::Pi) if first => call(p, Some(a), right),
            Math(crate::number::Math::Pi) => {
                let product = call(Arithmetic(Times), Some(right), a)?;
                call(Arithmetic(Divide), Some(&product), &Value::scalar(std::f64::consts::PI).unwrap())
            }
            Math(crate::number::Math::Log) if first => call(Math(crate::number::Math::Power), Some(a), right),
            Math(crate::number::Math::Log) => call(Math(crate::number::Math::Power), Some(a), &Arithmetic(Divide).call(None, right, span)?),
            Math(crate::number::Math::Circle) if first => {
                for e in a.elements() {
                    let code = numeric(&e, span)?.integer().map_err(|k| span.error(k, "circle inverse needs integer codes"))?;
                    if !(-7..=12).contains(&code) { return Err(span.error(ErrorKind::Domain, "circle code has no supported inverse")); }
                }
                let codes = Arithmetic(Minus).call(None, a, span)?;
                call(p, Some(&codes), right)
            }
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span)?), right),
            Transpose if first => {
                let perm = axes(a, right.shape().len(), span)?;
                if perm.len() != right.shape().len() { return Err(span.error(ErrorKind::Length, "inverse transpose needs an axis permutation")); }
                let mut inverse = vec![0; perm.len()];
                for (i, &axis) in perm.iter().enumerate() { inverse[axis] = i as i64 + 1; }
                p.call(Some(&Value::integers(vec![inverse.len()], inverse).unwrap()), right, span)
            }
            Decode if first => inverse_decode(a, right, span),
            Encode if first => Decode.call(Some(a), right, span),
            _ => Err(span.error(ErrorKind::Domain, "this bound function has no known inverse")),
        };
    }
    match p {
        Prime => {
            let below = p.call(Some(&Value::scalar(Number::from_integer(-1)).unwrap()), right, span)?;
            Arithmetic(Plus).call(Some(&Value::scalar(Number::from_integer(1)).unwrap()), &below, span)
        }
        Factor => crate::number_theory::product(right, span),
        Polynomial => p.call(None, right, span),
        Arithmetic(Plus | Minus | Divide) | Reverse(_) | Transpose | Identity(_) | Index | MatrixDivide => p.call(None, right, span),
        Math(crate::number::Math::Power) => Math(crate::number::Math::Log).call(None, right, span),
        Math(crate::number::Math::Log) => Math(crate::number::Math::Power).call(None, right, span),
        Math(crate::number::Math::Pi) => Arithmetic(Divide).call(Some(right), &Value::scalar(std::f64::consts::PI).unwrap(), span),
        Math(crate::number::Math::Circle) => {
            let log = Math(crate::number::Math::Log).call(None, right, span)?;
            Arithmetic(Times).call(Some(&Value::scalar(num_complex::Complex64::new(0., -1.)).unwrap()), &log, span)
        }
        Math(crate::number::Math::Root) => Math(crate::number::Math::Nand).call(None, right, span),
        Math(crate::number::Math::Nand) => Math(crate::number::Math::Root).call(None, right, span),
        Math(crate::number::Math::Nor) => Arithmetic(Divide).call(Some(right), &integer(2), span),
        Compare(Comparison::LessEqual) => Arithmetic(Plus).call(Some(right), &integer(1), span),
        Compare(Comparison::GreaterEqual) => Arithmetic(Minus).call(Some(right), &integer(1), span),
        Encode => Decode.call(None, right, span),
        Decode => Encode.call(None, right, span),
        Enclose => Take.call(None, right, span),
        Take => Enclose.call(None, right, span),
        Nest => Ok(if right.is_scalar() { right.disclose().clone() } else { right.clone() }),
        Iota => {
            let counter = if right.shape().len() == 1 && matches!(right.prototype(), Value::Number(_)) { Tally } else { Shape };
            let candidate = counter.call(None, right, span)?;
            if !array_match(&iota(&candidate, span)?, right, span)? { return Err(span.error(ErrorKind::Domain, "argument is not an index generator result")); }
            Ravel.call(None, &candidate, span)
        }
        Mix => split(right, None, span),
        Drop => Mix.call(None, right, span),
        Where => inverse_where(right, span),
        _ => Err(span.error(ErrorKind::Domain, "this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

fn inverse_decode(base: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
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
    let bases =
        Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).map_err(|k| span.error(k, "invalid inverse decode base"))?;
    radix(&bases, right, true, span)
}

fn inverse_where(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
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
    Value::integers(shape, data).map_err(|k| span.error(k, "invalid inverse where"))
}

fn matrix_divide(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use faer::{linalg::solvers::SolveLstsq, Mat};
    let dimensions = |a: &Value| match a.shape() {
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
    if m < n { return Err(span.error(ErrorKind::Length, "matrix is underdetermined")); }
    let mut shape = right.shape().get(1..).unwrap_or(&[]).to_vec();
    if let Some(x) = left { shape.extend_from_slice(x.shape().get(1..).unwrap_or(&[])); }
    else { shape = right.shape().iter().rev().copied().collect(); }
    generated_len(&[n, k]).map_err(|e| span.error(e, "matrix result is too large"))?;
    let numbers = |a: &Value| -> Result<Vec<Number>, Error> {
        numeric(&a.prototype(), span)?;
        a.elements()
            .map(|e| {
                let n = numeric(&e, span)?;
                if n.is_infinite() { return Err(span.error(ErrorKind::Domain, "matrix divide requires finite entries")); }
                Ok(n.clone())
            })
            .collect()
    };
    let a = numbers(right)?;
    let b = left.map(numbers).transpose()?;
    let exact = right.is_exact() && left.is_none_or(Value::is_exact);
    let prototype = if exact { right.prototype().clone() } else { float(0.) };
    if n == 0 { return Value::empty(shape, prototype).map_err(|e| span.error(e, "invalid matrix shape")); }
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
    if shape.is_empty() && (right.is_atom() || !right.is_scalar()) && left.is_none_or(|x| x.is_atom() || !x.is_scalar()) {
        return Ok(Value::Number(values[0].clone()));
    }
    Value::from_parts(shape, values.into_iter().map(Value::Number).collect(), prototype).map_err(|e| span.error(e, "invalid matrix result"))
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

fn binary_encode(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use num_bigint::BigInt;
    use num_traits::{FromPrimitive, Signed};
    let invalid = || span.error(ErrorKind::Domain, "binary encoding needs nonnegative integers");
    let mut numbers = Vec::with_capacity(right.len());
    let mut width = 0;
    for item in right.elements() {
        span.check()?;
        let n = numeric(&item, span)?;
        let value = if let Some(q) = n.as_exact() {
            if !q.is_integer() || q.is_negative() { return Err(invalid()); }
            q.to_integer()
        } else {
            let z = n.to_complex().map_err(|_| invalid())?;
            if z.im != 0.0 || z.re < 0.0 || z.re.fract() != 0.0 { return Err(invalid()); }
            BigInt::from_f64(z.re).ok_or_else(invalid)?
        };
        width = width.max(value.bits() as usize);
        numbers.push((value, n.clone()));
    }
    let shape = [&[width], right.shape()].concat();
    let mut data = Vec::with_capacity(generated_len(&shape).map_err(|k| span.error(k, "binary encoding is too large"))?);
    for bit in (0..width).rev() {
        span.check()?;
        data.extend(numbers.iter().map(|(n, domain)| Value::Number(domain.unit(i32::from(n.bit(bit as u64))))));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.unit(0));
    Value::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid binary encoding"))
}

fn radix(left: &Value, right: &Value, encode: bool, span: &Context<'_>) -> Result<Value, Error> {
    let numbers = |a: &Value| a.elements().map(|e| numeric(&e, span).cloned()).collect::<Result<Vec<_>, _>>();
    let (xs, ys) = (numbers(left)?, numbers(right)?);
    let zero = numeric(&right.prototype(), span)?.result_zero(Some(numeric(&left.prototype(), span)?));
    let error = |m| span.error(ErrorKind::Domain, m);
    if encode {
        let shape = [left.shape(), right.shape()].concat();
        let count = generated_len(&shape).map_err(|k| span.error(k, "encode result is too large"))?;
        let rows = left.shape().first().copied().unwrap_or(1);
        let columns = crate::array::element_count(left.shape().get(1..).unwrap_or(&[])).map_err(|k| span.error(k, "invalid radix shape"))?;
        let mut result = vec![Value::Number(zero.clone()); count];
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
                        result[(row * columns + column) * ys.len() + j] = Value::Number(digit);
                    }
                }
            }
        }
        let frame = if left.is_atom() && right.is_atom() { Frame::Direct } else { Frame::Array(shape) };
        return frame.collect(result, Value::Number(zero)).map_err(|k| span.error(k, "invalid encode result"));
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
        data.push(Value::Number(value));
    }
    if shape.is_empty() { return Ok(data.remove(0)); }
    Value::from_parts(shape, data, Value::Number(zero)).map_err(|k| span.error(k, "invalid decode result"))
}

fn element_order(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Function(_), _) | (_, Value::Function(_)) => unreachable!("ordering rejects function arrays"),
        (Value::Number(x), Value::Number(y)) => x.grade_order(y),
        (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) => Ordering::Less,
        (Value::Character(_), Value::Number(_)) => Ordering::Greater,
        (x @ Value::Array(_), y @ Value::Array(_)) => array_order(x, y),
        (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less,
    }
}

fn array_order(left: &Value, right: &Value) -> Ordering {
    let rank_order = left.shape().len().cmp(&right.shape().len());
    if !rank_order.is_eq() { return rank_order; }
    for (x, y) in left.elements().zip(right.elements()) {
        let order = element_order(&x, &y);
        if !order.is_eq() { return order; }
    }
    left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape()))
}

fn grade(left: Option<&Value>, right: &Value, down: bool, span: &Context<'_>) -> Result<Value, Error> {
    if right.has_functions() || left.is_some_and(Value::has_functions) { return Err(span.error(ErrorKind::Domain, "functions have no ordering")); }
    if right.is_scalar() || left.is_some_and(Value::is_scalar) { return Err(span.error(ErrorKind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1]).map_err(|k| span.error(k, "grade result is too large"))?;
    let mut indices: Vec<usize> = (0..count).collect();
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Value| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Value::Character(_)) || a.elements().any(|e| !matches!(e, Value::Character(_))) {
                return Err(span.error(ErrorKind::Domain, "dyadic grade needs simple character arrays"));
            }
            Ok(a.elements()
                .map(|e| match e { Value::Character(c) => c, _ => unreachable!() })
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
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid grade cells"))?;
        indices.sort_by(|&a, &b| direction(array_order(&cells[a], &cells[b])));
    }
    Value::integers(vec![indices.len()], indices.into_iter().map(|i| (i + 1) as i64).collect()).map_err(|k| span.error(k, "invalid grade result"))
}

fn interval_index(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.has_functions() || right.has_functions() { return Err(span.error(ErrorKind::Domain, "functions have no ordering")); }
    let SearchCells { left: boundaries, right: values, frame } = search_cells(left, right, span)?;
    for pair in boundaries.windows(2) {
        span.check()?;
        if array_order(&pair[0], &pair[1]).is_gt() { return Err(span.error(ErrorKind::Domain, "interval boundaries must be sorted")); }
    }
    let mut data = Vec::with_capacity(values.len());
    for value in &values {
        let (mut lo, mut hi) = (0, boundaries.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            span.check()?;
            if array_order(&boundaries[mid], value).is_gt() { hi = mid; } else { lo = mid + 1; }
        }
        data.push(generated(lo, true));
    }
    frame.collect(data, integer(0)).map_err(|k| span.error(k, "invalid interval index"))
}

fn roll(right: &Value, exact: bool, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
    let fill = fill || right.is_empty();
    let mut rng = rand::rng();
    let mut item = |e| {
        if let a @ Value::Array(_) = e { return roll(&a, exact, span, fill); }
        if fill { return Ok(generated(0, exact)); }
        let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "roll needs a nonnegative integer"))?;
        if !exact && n > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "roll bound exceeds exact floating-point integers")); }
        Ok(if n == 0 { float(rng.sample(rand::distr::Open01)) } else { generated(rng.random_range(1..=n), exact) })
    };
    let data = right.elements().map(&mut item).collect::<Result<Vec<_>, _>>()?;
    let prototype = if right.is_empty() { item(right.prototype().clone())? } else { generated(0, exact) };
    Frame::of(right).collect(data, prototype).map_err(|k| span.error(k, "invalid roll result"))
}

fn deal(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let count = |a: &Value| {
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
    Value::from_parts(vec![n], data, generated(0, exact)).map_err(|k| span.error(k, "invalid deal result"))
}

fn scalar_axes(p: Primitive, left: &Value, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let left_small = left.shape().len() < right.shape().len();
    let (small, large) = if left_small { (left, right) } else { (right, left) };
    let axes = axes(spec, large.shape().len(), span)?;
    if axes.len() != small.shape().len() { return Err(span.error(ErrorKind::Length, "scalar-function axes must match the lower rank")); }
    let agreement = Agreement::with_axes(left.shape(), right.shape(), &axes).map_err(|k| span.error(k, "scalar-function axis lengths differ"))?;
    p.scalar_mapped(Some(left), right, span, false, &agreement)
}

fn reorder(right: &Value, order: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let mut labels = vec![0; order.len()];
    for (i, &axis) in order.iter().enumerate() { labels[axis] = (i + 1) as i64; }
    transpose(Some(&Value::integers(vec![labels.len()], labels).unwrap()), right, span)
}

fn enclose_axes(right: &Value, axes: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let mut order: Vec<_> = (0..right.shape().len()).filter(|a| !axes.contains(a)).collect();
    order.extend(axes);
    let permuted = reorder(right, &order, span)?;
    let cells = permuted.cells(axes.len()).map_err(|k| span.error(k, "invalid enclosed cells"))?;
    let data: Vec<_> = cells.collect().map_err(|k| span.error(k, "invalid enclosed cells"))?.into_iter().collect();
    let prototype = if data.is_empty() { cells.prototype().map_err(|k| span.error(k, "invalid enclosed prototype"))? } else { data[0].prototype() };
    Value::from_parts(cells.frame().to_vec(), data, prototype).map_err(|k| span.error(k, "invalid enclosed array"))
}

fn mix_axes(right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mixed = Primitive::Mix.call(None, right, span)?;
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

fn laminate(left: &Value, right: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let shape = if left.is_scalar() { right.shape() } else { left.shape() };
    if !left.is_scalar() && !right.is_scalar() && left.shape() != right.shape() {
        return Err(span.error(ErrorKind::Length, "laminate argument shapes differ"));
    }
    let dimensions = Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).unwrap();
    let extend = |a: &Value| {
        let mut extended = shape.to_vec();
        extended.insert(axis, 1);
        let a = if a.is_scalar() { reshape(&dimensions, a, span)? } else { a.clone() };
        a.with_shape(extended).map_err(|k| span.error(k, "invalid laminate shape"))
    };
    catenate(&extend(left)?, &extend(right)?, Some(axis), false, span)
}

fn reshape(dimensions: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "shape must be a scalar or vector")); }
    let shape = dimensions
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid dimension")))
        .collect::<Result<Vec<_>, _>>()?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "shape exceeds array limits"))?;
    if let Some(values) = right.as_floats() {
        let data = if values.is_empty() { vec![0.0; len] } else { values.iter().copied().cycle().take(len).collect() };
        return Value::floats(shape, data).map_err(|k| span.error(k, "invalid reshape"));
    }
    if let Some(values) = right.as_integers() {
        let data = if values.is_empty() { vec![0; len] } else { values.iter().copied().cycle().take(len).collect() };
        return Value::integers(shape, data).map_err(|k| span.error(k, "invalid reshape"));
    }
    let data = if right.is_empty() { vec![right.prototype().clone(); len] } else { right.elements().cycle().take(len).collect() };
    Value::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid reshape"))
}

fn remap(right: &Value, shape: Vec<usize>, source: impl Fn(usize) -> Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
    let len = generated_len(&shape).map_err(|k| span.error(k, "result exceeds array limits"))?;
    if right.is_atom() && shape.is_empty() { return Ok(source(0).map_or_else(|| right.prototype(), |i| right.at(i))); }
    if let Some(values) = right.as_floats() {
        return Value::floats(shape, (0..len).map(|i| source(i).map_or(0.0, |j| values[j])).collect()).map_err(|k| span.error(k, "invalid structural result"));
    }
    if let Some(values) = right.as_integers() {
        return Value::integers(shape, (0..len).map(|i| source(i).map_or(0, |j| values[j])).collect()).map_err(|k| span.error(k, "invalid structural result"));
    }
    let data = (0..len).map(|i| source(i).map_or_else(|| right.prototype().clone(), |j| right.at(j))).collect();
    Value::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid structural result"))
}

fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "take/drop counts must be a scalar or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let old = if right.is_scalar() { vec![1; counts.len()] } else { right.shape().to_vec() };
    let mut shape = old.clone();
    let mut starts = vec![0i128; shape.len()];
    if counts.len() > shape.len() { return Err(span.error(ErrorKind::Rank, "counts exceed argument rank")); }
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

fn replicate(counts: &Value, right: &Value, first: bool, axis: Option<usize>, expand: bool, span: &Context<'_>) -> Result<Value, Error> {
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
    Value::from_parts(shape, data, right.prototype().clone()).map_err(|k| span.error(k, "invalid replication result"))
}

fn rotate(counts: Option<&Value>, right: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
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

fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(span.error(ErrorKind::Domain, "catenate axis is outside result rank")); }
    let promote = |a: &Value, other: &Value| -> Result<Vec<usize>, Error> {
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
    Value::from_parts(shape, data, left.prototype().clone()).map_err(|k| span.error(k, "invalid catenate result"))
}

fn transpose(axes: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let axes = match axes {
        None => (0..rank).rev().collect::<Vec<_>>(),
        Some(a) => {
            if a.len() != rank { return Err(span.error(ErrorKind::Length, "transpose needs one axis per dimension")); }
            a.elements()
                .map(|e| {
                    let n = numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid transpose axis"))?;
                    if n == 0 { return Err(span.error(ErrorKind::Domain, "transpose axes start at one")); }
                    if n > rank { return Err(span.error(ErrorKind::Rank, "transpose axis exceeds argument rank")); }
                    Ok(n - 1)
                })
                .collect::<Result<_, _>>()?
        }
    };
    let mut shape = vec![usize::MAX; axes.iter().max().map_or(0, |n| n + 1)];
    for (i, &axis) in axes.iter().enumerate() { shape[axis] = shape[axis].min(right.shape()[i]); }
    if shape.contains(&usize::MAX) { return Err(span.error(ErrorKind::Rank, "transpose axes must be consecutive from 1")); }
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

fn split(right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_scalar() { return Value::new(vec![], vec![right.clone()]).map_err(|k| span.error(k, "invalid split result")); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    let traversal = Axis::new(right.shape(), axis).map_err(|_| span.error(ErrorKind::Domain, "split axis is outside array rank"))?;
    let mut frame = right.shape().to_vec();
    frame.remove(axis);
    let size = generated_len(&frame).map_err(|k| span.error(k, "split exceeds array limits"))?;
    generated_len(&[traversal.len]).map_err(|k| span.error(k, "split cell exceeds array limits"))?;
    let prototype = Value::from_parts(vec![traversal.len], vec![right.prototype().clone(); traversal.len], right.prototype().clone()).unwrap();
    let mut data = Vec::with_capacity(size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for k in 0..traversal.inner {
            let cell = (0..traversal.len).map(|j| right.at(traversal.offset(i, j, k))).collect();
            data.push(Value::from_parts(vec![traversal.len], cell, right.prototype().clone()).unwrap());
        }
    }
    Value::from_parts(frame, data, prototype).map_err(|k| span.error(k, "invalid split result"))
}

fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool, span: &Context<'_>) -> Result<Value, Error> {
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
        let prototype = Value::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid partition prototype"))?;
        let mut data = Vec::with_capacity(ranges.len());
        for range in ranges {
            let mut shape = right.shape().to_vec();
            shape[axis] = range.len();
            let width = range.len() * traversal.inner;
            let cell = remap(&right, shape, |i| Some(i / width * traversal.len * traversal.inner + range.start * traversal.inner + i % width), span)?;
            data.push(cell);
        }
        return Value::from_parts(vec![data.len()], data, prototype).map_err(|k| span.error(k, "invalid partition result"));
    }
    let mut shape = right.shape().to_vec();
    shape[axis] = ranges.len();
    let size = generated_len(&shape).map_err(|k| span.error(k, "partition result is too large"))?;
    let prototype = Value::empty(vec![0], right.prototype().clone()).unwrap();
    let mut data = Vec::with_capacity(size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for range in &ranges {
            for k in 0..traversal.inner {
                let items = range.clone().map(|j| right.at(traversal.offset(i, j, k))).collect();
                let cell = Value::from_parts(vec![range.len()], items, right.prototype().clone()).map_err(|e| span.error(e, "invalid partition cell"))?;
                data.push(cell);
            }
        }
    }
    Value::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid partition result"))
}

fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "squad indices must be a scalar or vector")); }
    if left.len() > right.shape().len() || axes.is_some_and(|a| a.len() != left.len()) {
        return Err(span.error(ErrorKind::Length, "squad needs one index item per selected axis"));
    }
    let mut parts = vec![None; right.shape().len()];
    for (i, item) in left.elements().enumerate() {
        let coords = item.clone();
        for n in coords.elements() { numeric(&n, span)?; }
        let part = parts.get_mut(axes.map_or(i, |a| a[i])).ok_or_else(|| span.error(ErrorKind::Rank, "squad axis is outside array rank"))?;
        *part = Some(coords);
    }
    select(right, &parts, span)
}

fn coordinate_offset(coords: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Option<usize>, Error> {
    if coords.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a coordinate must be a scalar or vector")); }
    if coords.elements().any(|e| !matches!(e, Value::Number(_))) { return Err(span.error(ErrorKind::Domain, "a coordinate must contain numbers")); }
    if coords.len() != right.shape().len() { return Err(span.error(ErrorKind::Rank, "a coordinate needs one index per axis")); }
    let mut offset = Some(0);
    for (n, &size) in coords.elements().zip(right.shape()) {
        let n = numeric(&n, span)?.integer().map_err(|k| span.error(k, "index must be an integer"))?;
        let outside = n <= 0 || n as usize > size;
        if n < 0 || (outside && !prototype) { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
        offset = if outside { None } else { offset.map(|offset| offset * size + n as usize - 1) };
    }
    Ok(offset)
}

pub(crate) fn pick(left: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "Pick needs one coordinate field per axis")); }
    if left.is_empty() { return Ok(right.clone()); }
    let source = if right.is_scalar() { &[1][..] } else { right.shape() };
    let fields: Vec<_> = left.elements().collect();
    if fields.len() > source.len() { return Err(span.error(ErrorKind::Rank, "too many Pick coordinates")); }
    let mut frame = Vec::new();
    for field in &fields { frame = Agreement::new(&frame, field.shape()).map_err(|k| span.error(k, "Pick coordinate fields do not agree"))?.shape; }
    let maps = fields.iter().map(|f| Agreement::new(f.shape(), &frame).unwrap().left).collect::<Vec<_>>();
    let trailing = &source[fields.len()..];
    let cell_len = crate::array::element_count(trailing).map_err(|k| span.error(k, "invalid Pick cell"))?;
    let shape = [&frame, trailing].concat();
    let len = generated_len(&shape).map_err(|k| span.error(k, "Pick result is too large"))?;
    let mut values = Vec::with_capacity(len);
    for i in 0..crate::array::element_count(&frame).unwrap() {
        let mut offset = Some(0);
        for (axis, (field, map)) in fields.iter().zip(&maps).enumerate() {
            let n = numeric(&field.at(map.index(i)), span)?.integer().map_err(|k| span.error(k, "Pick coordinates must be integers"))?;
            let size = source[axis];
            if n <= 0 || n as usize > size {
                if n < 0 || !(prototype || size == 0 && n == 1) { return Err(span.error(ErrorKind::Index, "Pick coordinate is outside the array")); }
                offset = None;
            } else { offset = offset.map(|o| o * size + n as usize - 1); }
        }
        for j in 0..cell_len { values.push(offset.map_or_else(|| right.prototype(), |o| right.at(o * cell_len + j))); }
    }
    let frame = if trailing.is_empty() && fields.iter().all(Value::is_atom) { Frame::Direct } else { Frame::Array(shape) };
    frame.collect(values, right.prototype()).map_err(|k| span.error(k, "invalid Pick result"))
}

pub(crate) fn index(n: &Number, limit: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().map_err(|k| span.error(k, "index must be a positive integer"))?;
    if n <= 0 || n as usize > limit { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
    Ok(n as usize - 1)
}

pub(crate) struct Selection { pub frame: Frame, pub paths: Vec<Vec<usize>> }

impl Selection {
    pub(crate) fn values<'a>(&'a self, values: &'a Value, span: &Context<'_>) -> Result<impl Iterator<Item = Value> + 'a, Error> {
        if matches!(self.frame, Frame::Array(_)) && !values.is_singleton() && values.shape() != self.frame.shape() {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        Ok((0..self.paths.len()).map(|i| if matches!(self.frame, Frame::Direct) { values.clone() } else { selected(values, i) }))
    }

    pub(crate) fn read(&self, array: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let data = self
            .paths
            .iter()
            .map(|path| {
                let mut item = array.clone();
                for &i in path { item = item.clone().at(i); }
                item
            })
            .collect();
        self.frame.clone().collect(data, array.prototype()).map_err(|k| span.error(k, "invalid selection"))
    }

    pub(crate) fn write(&self, array: &Value, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        fn replace(array: &Value, updates: &[(&[usize], Value)], span: &Context<'_>) -> Result<Value, Error> {
            let replacement;
            let (array, updates) = if let Some(last) = updates.iter().rposition(|(p, _)| p.is_empty()) {
                replacement = updates[last].1.clone();
                (&replacement, &updates[last + 1..])
            } else { (array, updates) };
            if updates.is_empty() { return Ok(array.clone()); }
            let mut data: Vec<_> = array.elements().collect();
            let mut groups: HashMap<usize, Vec<(&[usize], Value)>> = HashMap::new();
            for (path, value) in updates { groups.entry(path[0]).or_default().push((&path[1..], value.clone())); }
            for (i, edits) in groups {
                if i >= data.len() { return Err(span.error(ErrorKind::Index, "replacement changed a selected path")); }
                data[i] = replace(&data[i].clone(), &edits, span)?;
            }
            if array.is_atom() { return Ok(data.remove(0)); }
            Value::from_parts(array.shape().to_vec(), data, array.prototype().clone()).map_err(|k| span.error(k, "invalid amended array"))
        }
        let updates: Vec<_> = self.paths.iter().map(Vec::as_slice).zip(self.values(values, span)?).collect();
        replace(array, &updates, span)
    }
}

pub(crate) fn choose(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    let mut paths = Vec::with_capacity(indices.len());
    for item in indices.elements() {
        let coordinates = item.clone();
        let reach = coordinates.elements().any(|e| matches!(e, Value::Array(_))) || (right.shape().len() == 1 && coordinates.len() > 1);
        let steps = if reach { coordinates.elements().collect() } else { vec![coordinates] };
        let mut current = right.clone();
        let mut path = Vec::new();
        for coords in steps {
            let offset = coordinate_offset(&coords, &current, false, span)?.unwrap();
            path.push(offset);
            current = current.at(offset).clone();
        }
        paths.push(path);
    }
    Ok(Selection { frame: Frame::of(indices), paths })
}

pub(crate) fn at_indices(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    if matches!(indices.elements().next().unwrap_or_else(|| indices.prototype().clone()), Value::Array(_)) { return choose(right, indices, span); }
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
    let frame = if indices.is_atom() && right.shape().len() == 1 { Frame::Direct } else { Frame::Array(shape) };
    Ok(Selection { frame, paths })
}

pub(crate) fn select(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Value, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    selection(right, parts, span)?.read(right, span)
}

pub(crate) fn selection(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() { return Ok(Selection { frame: Frame::Direct, paths: vec![vec![]] }); }
    if let [Some(indices)] = parts {
        if matches!(indices.elements().next().unwrap_or_else(|| indices.prototype().clone()), Value::Array(_)) { return choose(right, indices, span); }
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
    let frame = if parts.iter().all(|a| a.as_ref().is_some_and(Value::is_atom)) { Frame::Direct } else { Frame::Array(shape) };
    Ok(Selection { frame, paths })
}
