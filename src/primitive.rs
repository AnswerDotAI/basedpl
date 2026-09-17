use crate::{number::Arithmetic, Array, Element, Error, ErrorKind, Number, Span};

pub(crate) const MAX_GENERATED_ELEMENTS: usize = 1_000_000;

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
    Compare(Comparison),
    Iota,
    Shape,
    Tally,
    Ravel,
    Replicate,
}

fn numeric<'a>(e: &'a Element, span: &Span) -> Result<&'a Number, Error> {
    match e { Element::Number(n) => Ok(n), _ => Err(span.error(ErrorKind::Domain, "expected numeric elements")) }
}
fn float(n: f64) -> Element { Element::Number(Number::try_from(n).expect("finite generated number")) }
fn selected(a: &Array, i: usize) -> &Element { &a.data()[if a.is_singleton() { 0 } else { i }] }

/// Singleton extension is shared selection, not a universal broadcasting policy.
/// Scalar functions preserve shape; replicate below agrees along its selected axis.
fn scalar_agreement<'a>(left: Option<&'a Array>, right: &'a Array, span: &Span) -> Result<&'a Array, Error> {
    let Some(left) = left else { return Ok(right); };
    if left.is_singleton() && right.is_singleton() { return Ok(if left.shape().len() > right.shape().len() { left } else { right }); }
    if left.is_singleton() { return Ok(right); }
    if right.is_singleton() { return Ok(left); }
    if left.shape() != right.shape() { return Err(span.error(ErrorKind::Length, "array shapes do not agree")); }
    Ok(right)
}

impl Primitive {
    pub(crate) fn from_glyph(c: char) -> Option<Self> {
        use Arithmetic::*;
        use Comparison::*;
        Some(match c {
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
            '⍴' => Self::Shape,
            '≢' => Self::Tally,
            ',' => Self::Ravel,
            _ => return None,
        })
    }
    pub(crate) fn call(self, left: Option<&Array>, right: &Array, span: &Span) -> Result<Array, Error> {
        let construct = |shape, data| Array::from_parts(shape, data, float(0.0)).map_err(|k| span.error(k, "invalid array result"));
        match self {
            Self::Replicate => return replicate(left.ok_or_else(|| span.error(ErrorKind::Syntax, "replicate needs a left argument"))?, right, span),
            Self::Iota | Self::Shape | Self::Tally | Self::Ravel => {
                if left.is_some() { return Err(span.error(ErrorKind::Unsupported, "this dyadic primitive is not implemented yet")); }
                return match self {
                    Self::Iota => {
                        let n = right.as_number().ok_or_else(|| span.error(ErrorKind::Rank, "iota requires a numeric scalar count"))?;
                        let n = n.nonnegative_integer().map_err(|k| span.error(k, "count must be a representable nonnegative integer"))?;
                        if n > MAX_GENERATED_ELEMENTS { return Err(span.error(ErrorKind::Limit, "iota result exceeds 1000000 elements")); }
                        construct(vec![n], (1..=n).map(|i| float(i as f64)).collect())
                    }
                    Self::Shape => construct(vec![right.shape().len()], right.shape().iter().map(|&n| float(n as f64)).collect()),
                    Self::Tally => Array::scalar(right.shape().first().copied().unwrap_or(1) as f64).map_err(|k| span.error(k, "invalid tally")),
                    Self::Ravel => Array::from_parts(vec![right.data().len()], right.data().to_vec(), right.prototype().clone())
                        .map_err(|k| span.error(k, "invalid ravel")),
                    _ => unreachable!(),
                };
            }
            _ => (),
        }
        if matches!(self, Self::Compare(_)) && left.is_none() {
            return Err(span.error(ErrorKind::Unsupported, "this monadic primitive is not implemented yet"));
        }
        let model = scalar_agreement(left, right, span)?;
        if model.data().is_empty() {
            let y = numeric(right.prototype(), span)?;
            let x = left.map(|a| numeric(a.prototype(), span)).transpose()?;
            let prototype = if matches!(self, Self::Compare(_)) { float(0.0) } else { Element::Number(y.result_zero(x)) };
            return Array::empty(model.shape().to_vec(), prototype).map_err(|k| span.error(k, "invalid empty result"));
        }
        let mut data = Vec::with_capacity(model.data().len());
        for i in 0..model.data().len() {
            let y = numeric(selected(right, i), span)?;
            let x = left.map(|a| numeric(selected(a, i), span)).transpose()?;
            let n = match self {
                Self::Arithmetic(op) => match x { Some(x) => x.dyad(op, y), None => y.monad(op) },
                Self::Compare(op) => {
                    let order = x.unwrap().compare(y);
                    order.map(|order| {
                        use Comparison::*;
                        let b = match op {
                            Equal => order.is_eq(),
                            NotEqual => !order.is_eq(),
                            Less => order.is_lt(),
                            LessEqual => !order.is_gt(),
                            Greater => order.is_gt(),
                            GreaterEqual => !order.is_lt(),
                        };
                        Number::try_from(u8::from(b) as f64).unwrap()
                    })
                }
                _ => unreachable!(),
            }
            .map_err(|message| span.error(ErrorKind::Domain, message))?;
            data.push(Element::Number(n));
        }
        Array::new(model.shape().to_vec(), data).map_err(|k| span.error(k, "invalid array result"))
    }
}

fn replicate(counts: &Array, right: &Array, span: &Span) -> Result<Array, Error> {
    if counts.shape().len() > 1 || right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "replicate currently accepts scalars and vectors")); }
    if !counts.is_singleton() && !right.is_singleton() && counts.data().len() != right.data().len() {
        return Err(span.error(ErrorKind::Length, "replication counts and data do not agree"));
    }
    let len = if right.is_singleton() { counts.data().len() } else { right.data().len() };
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts: Vec<_> = counts
        .data()
        .iter()
        .map(|e| numeric(e, span)?.nonnegative_integer().map_err(|k| span.error(k, "replication count must be a representable nonnegative integer")))
        .collect::<Result<_, _>>()?;
    let mut data = Vec::new();
    for i in 0..len {
        let n = counts[if counts.len() == 1 { 0 } else { i }];
        if n > MAX_GENERATED_ELEMENTS - data.len() { return Err(span.error(ErrorKind::Limit, "replication result exceeds 1000000 elements")); }
        data.extend(std::iter::repeat_n(selected(right, i).clone(), n));
    }
    Array::from_parts(vec![data.len()], data, right.prototype().clone()).map_err(|k| span.error(k, "invalid replication result"))
}
