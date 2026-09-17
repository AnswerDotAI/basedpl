use crate::{
    agreement::Agreement,
    array::{generated_len, Axis},
    primitive::{Hybrid, OperatorKind, Primitive},
    syntax::{Definition, DefinitionKind, Node, NodeKind},
    Array, Element, Error, ErrorKind, ParseStatus, Parsed, Source, Span,
};
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, Debug)]
struct Function { node: Rc<FunctionNode>, depth: usize, environment: Option<usize> }

const MAX_DEPTH: usize = 128;
const MAX_CALL_DEPTH: usize = 64;

#[derive(Debug)]
enum FunctionNode {
    Primitive(Primitive),
    Fold(Function, Hybrid),
    Inverse(Function),
    Axis(Function, Array),
    Defined(Closure),
    Derived(Closure, Operand, Option<Operand>),
    Modified(OperatorKind, Operand),
    Composed(OperatorKind, [Operand; 2]),
    Fork([Function; 3]),
}

impl Function {
    fn selects(&self, dyadic: bool) -> bool {
        use Primitive::*;
        match self.node.as_ref() {
            FunctionNode::Primitive(p) => match p {
                Ravel | CatenateFirst => !dyadic,
                Take | Drop | Shape | Replicate(_) | Expand(_) => dyadic,
                Member => !dyadic,
                Reverse(_) | Transpose | Disclose | Index => true,
                _ => false,
            },
            FunctionNode::Axis(f, _) | FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => f.selects(dyadic),
            _ => false,
        }
    }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        use crate::display::Tree;
        if *budget == 0 { return Tree::leaf("…"); }
        *budget -= 1;
        let (label, children) = match self.node.as_ref() {
            FunctionNode::Primitive(p) => return Tree::leaf(p.glyph().to_string()),
            FunctionNode::Defined(c) => return Tree::leaf(c.text()),
            FunctionNode::Fold(f, h) => (h.text(), vec![f.tree(budget)]),
            FunctionNode::Inverse(f) => ("⍣¯1".into(), vec![f.tree(budget)]),
            FunctionNode::Axis(f, a) => (format!("[{a}]"), vec![f.tree(budget)]),
            FunctionNode::Modified(op, a) => (op.glyph().into(), vec![a.tree(budget)]),
            FunctionNode::Composed(op, args) => (op.glyph().into(), args.iter().map(|a| a.tree(budget)).collect()),
            FunctionNode::Fork(fs) => ("fork".into(), fs.iter().map(|f| f.tree(budget)).collect()),
            FunctionNode::Derived(c, a, b) => (c.text().into(), std::iter::once(a).chain(b.iter()).map(|a| a.tree(budget)).collect()),
        };
        Tree { label, children }
    }
    fn text(&self, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        match self.node.as_ref() {
            FunctionNode::Primitive(p) => p.glyph().to_string(),
            FunctionNode::Defined(c) => c.text().into(),
            FunctionNode::Fold(f, h) => format!("({}){}", f.text(budget), h.text()),
            FunctionNode::Inverse(f) => format!("({})⍣¯1", f.text(budget)),
            FunctionNode::Axis(f, a) => format!("({})[{a}]", f.text(budget)),
            FunctionNode::Modified(op, a) => format!("({}){}", a.text(budget), op.glyph()),
            FunctionNode::Composed(op, [a, b]) => format!("({}){}({})", a.text(budget), op.glyph(), b.text(budget)),
            FunctionNode::Fork(fs) => fs.iter().map(|f| format!("({})", f.text(budget))).collect(),
            FunctionNode::Derived(c, a, b) => {
                format!("({}){}{}", a.text(budget), c.text(), b.as_ref().map_or(String::new(), |b| format!("({})", b.text(budget))))
            }
        }
    }
    fn train(mut functions: Vec<Self>, span: &Span) -> Result<Self, Error> {
        let mut result = functions.pop().unwrap();
        while functions.len() >= 2 {
            let middle = functions.pop().unwrap();
            result = Self::new(FunctionNode::Fork([functions.pop().unwrap(), middle, result]), span)?;
        }
        if let Some(first) = functions.pop() {
            result = Self::new(FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(first), Operand::Function(result)]), span)?;
        }
        Ok(result)
    }
    fn primitive(p: Primitive) -> Self { Self { node: Rc::new(FunctionNode::Primitive(p)), depth: 1, environment: None } }
    fn new(node: FunctionNode, span: &Span) -> Result<Self, Error> {
        let (depth, environment) = match &node {
            FunctionNode::Primitive(_) => (0, None),
            FunctionNode::Fold(f, _) | FunctionNode::Axis(f, _) | FunctionNode::Inverse(f) => (f.depth, f.environment),
            FunctionNode::Defined(c) => (0, c.environment),
            FunctionNode::Derived(c, a, b) => {
                let (depth, environment) = operand_dependencies(std::iter::once(a).chain(b.iter()));
                (depth, c.environment.max(environment))
            }
            FunctionNode::Composed(_, operands) => operand_dependencies(operands.iter()),
            FunctionNode::Modified(_, Operand::Function(f)) => (f.depth, f.environment),
            FunctionNode::Modified(_, _) => (0, None),
            FunctionNode::Fork(fs) => (fs.iter().map(|f| f.depth).max().unwrap(), fs.iter().filter_map(|f| f.environment).max()),
        };
        let depth = depth + 1;
        if depth > MAX_DEPTH { return Err(span.error(ErrorKind::Limit, "function structure exceeds 128 levels")); }
        Ok(Self { node: Rc::new(node), depth, environment })
    }
    fn call(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
        session.execution.check(span)?;
        if session.depth == MAX_CALL_DEPTH { return Err(span.error(ErrorKind::Limit, "evaluation depth exceeds 64")); }
        session.depth += 1;
        let result = self.apply(left, right, span, session, output);
        session.depth -= 1;
        session.execution.check(span)?;
        result
    }
    fn call_array(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Array, Error> {
        self.call(left, right, span, session, output)?.array(span)
    }
    fn inverse(&self, span: &Span) -> Result<Self, Error> {
        if let FunctionNode::Inverse(f) = self.node.as_ref() { Ok(f.clone()) } else { Self::new(FunctionNode::Inverse(self.clone()), span) }
    }
    fn apply(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
        use FunctionNode::{Defined, Derived, Fold, Fork};
        let array = match self.node.as_ref() {
            FunctionNode::Primitive(Primitive::Execute) => return session.execute(left, right, span, output),
            FunctionNode::Inverse(f) => return inverse(f, left, right, span, session, output),
            FunctionNode::Composed(op, operands) => return composition(*op, operands, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => return each(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Outer, Operand::Function(f)) => return outer(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Key, Operand::Function(f)) => return key(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Function(f)) => return f.call(Some(right), left.unwrap_or(right), span, session, output),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Array(a)) => Ok(a.clone()),
            FunctionNode::Modified(..) => unreachable!(),
            FunctionNode::Primitive(p) => p.call(left, right, &session.execution.at(span)),
            Defined(_) | Derived(..) => {
                return session.call_defined(self, left, right, output).map_err(|mut e| {
                    e.calls.push(span.clone());
                    e
                })
            }
            Fork(fns) => {
                let y = fns[2].call_array(left, right, span, session, output)?;
                let x = fns[0].call_array(left, right, span, session, output)?;
                return fns[1].call(Some(&x), &y, span, session, output);
            }
            Fold(operand, hybrid) => fold(operand, *hybrid, left, right, span, session, output),
            FunctionNode::Axis(f, axis) => {
                let mut f = f;
                while let FunctionNode::Axis(inner, _) = f.node.as_ref() { f = inner; }
                match f.node.as_ref() {
                    FunctionNode::Primitive(p) => p.call_axes(left, right, axis, &session.execution.at(span)),
                    Fold(operand, h) => {
                        fold(operand, Hybrid { axis: Some(crate::primitive::single_axis(axis, span)?), ..*h }, left, right, span, session, output)
                    }
                    _ => Err(span.error(ErrorKind::Unsupported, "axes on this function are not supported")),
                }
            }
        }?;
        Ok(Bound::new(Value::Array(array)))
    }
}

fn composition(
    op: OperatorKind,
    operands: &[Operand; 2],
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    use OperatorKind::*;
    if matches!(op, At) { return at(operands, left, right, span, session, output); }
    if matches!(op, Stencil) {
        let [Operand::Function(f), Operand::Array(spec)] = operands else {
            return Err(span.error(ErrorKind::Domain, "stencil needs a function and window specification"));
        };
        if left.is_some() { return Err(span.error(ErrorKind::Syntax, "stencil is monadic")); }
        return stencil(f, spec, right, span, session, output);
    }
    if matches!(op, Power) {
        let Operand::Function(f) = &operands[0] else { return Err(span.error(ErrorKind::Domain, "power needs a function left operand")); };
        return power(f, &operands[1], left, right, span, session, output);
    }
    match (&operands[0], &operands[1]) {
        (Operand::Array(a), Operand::Function(f)) if matches!(op, Compose) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "bound functions are monadic")); }
            f.call(Some(a), right, span, session, output)
        }
        (Operand::Function(f), Operand::Array(a)) if matches!(op, Compose) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "bound functions are monadic")); }
            f.call(Some(right), a, span, session, output)
        }
        (Operand::Function(f), Operand::Array(ranks)) if matches!(op, Rank) => rank(f, ranks, left, right, span, session, output),
        (Operand::Function(f), Operand::Function(g)) => match op {
            Product => inner(f, g, left, right, span, session, output),
            Compose => {
                let y = g.call_array(None, right, span, session, output)?;
                f.call(left, &y, span, session, output)
            }
            Rank => {
                let y = g.call_array(left, right, span, session, output)?;
                f.call(None, &y, span, session, output)
            }
            Over => {
                let y = g.call_array(None, right, span, session, output)?;
                let x = left.map(|x| g.call_array(None, x, span, session, output)).transpose()?;
                f.call(x.as_ref(), &y, span, session, output)
            }
            Behind => {
                let x = f.call_array(None, left.unwrap_or(right), span, session, output)?;
                g.call(Some(&x), right, span, session, output)
            }
            _ => unreachable!(),
        },
        _ => Err(span.error(ErrorKind::Domain, "invalid operator operands")),
    }
}

fn at(operands: &[Operand; 2], left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let selection = match &operands[1] {
        Operand::Array(indices) => crate::primitive::at_indices(right, indices, &session.execution.at(span))?,
        Operand::Function(f) => {
            let mask = f.call_array(None, right, span, session, output)?;
            if mask.shape() != right.shape() { return Err(span.error(ErrorKind::Length, "at mask must match argument shape")); }
            let mut paths = Vec::new();
            for (i, e) in mask.elements().enumerate() {
                let Element::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "at mask must be Boolean")); };
                if n.boolean().map_err(|m| span.error(ErrorKind::Domain, m))? { paths.push(vec![i]); }
            }
            crate::primitive::Selection { shape: vec![paths.len()], paths }
        }
        _ => unreachable!(),
    };
    let values = match &operands[0] {
        Operand::Array(a) => a.clone(),
        Operand::Function(f) => f.call_array(left, &selection.read(right, &session.execution.at(span))?, span, session, output)?,
        _ => unreachable!(),
    };
    let array = selection.write(right, &values, &session.execution.at(span))?;
    Ok(Bound::new(Value::Array(array)))
}

fn stencil(f: &Function, spec: &Array, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    if spec.shape().len() > 2 { return Err(span.error(ErrorKind::Rank, "stencil specification must be a scalar, vector or two-row matrix")); }
    let axes = if spec.shape().len() == 2 {
        if spec.shape()[0] != 2 { return Err(span.error(ErrorKind::Length, "stencil matrix needs two rows")); }
        spec.shape()[1]
    } else { spec.len() };
    if axes == 0 || axes > right.shape().len() { return Err(span.error(ErrorKind::Domain, "stencil needs at least one axis, within the argument rank")); }
    let mut sizes = Vec::new();
    for e in spec.elements() {
        let Element::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "stencil sizes must be numeric")); };
        let n = n.nonnegative_integer().map_err(|k| span.error(k, "stencil sizes must be positive integers"))?;
        if n == 0 { return Err(span.error(ErrorKind::Domain, "stencil sizes must be positive")); }
        sizes.push(n);
    }
    let (windows, movements) = sizes.split_at(axes);
    let mut frame = Vec::new();
    for (axis, &window) in windows.iter().enumerate() {
        let len = right.shape()[axis];
        if window / 2 >= len { return Err(span.error(ErrorKind::Domain, "stencil window is too large for the argument")); }
        frame.push((len - usize::from(window % 2 == 0)).div_ceil(movements.get(axis).copied().unwrap_or(1)));
    }
    let shape = [windows, &right.shape()[axes..]].concat();
    let size = generated_len(&shape).map_err(|k| span.error(k, "stencil window is too large"))?;
    let count = generated_len(&frame).map_err(|k| span.error(k, "stencil result is too large"))?;
    let mut results = Vec::with_capacity(count);
    for mut position in 0..count {
        let mut starts = vec![0isize; axes];
        let mut padding = vec![0.; axes];
        for a in (0..axes).rev() {
            starts[a] = ((position % frame[a]) * movements.get(a).copied().unwrap_or(1)) as isize - ((windows[a] - 1) / 2) as isize;
            position /= frame[a];
            padding[a] = if starts[a] < 0 { -starts[a] } else { (right.shape()[a] as isize - starts[a] - windows[a] as isize).min(0) } as f64;
        }
        let mut data = Vec::with_capacity(size);
        for mut flat in 0..size {
            let (mut offset, mut stride, mut valid) = (0, 1, true);
            for a in (0..shape.len()).rev() {
                let coordinate = (flat % shape[a]) as isize + starts.get(a).copied().unwrap_or(0);
                flat /= shape[a];
                if coordinate < 0 || coordinate >= right.shape()[a] as isize { valid = false; }
                else { offset += coordinate as usize * stride; }
                stride *= right.shape()[a];
            }
            data.push(if valid { right.at(offset) } else { right.prototype().clone() });
        }
        let window = Array::from_parts(shape.clone(), data, right.prototype().clone()).map_err(|k| span.error(k, "invalid stencil window"))?;
        let border = Array::floats(vec![axes], padding).unwrap();
        results.push(f.call_array(Some(&border), &window, span, session, output)?);
    }
    let result = Array::assemble(&frame, &results, &Array::scalar(0.).unwrap()).map_err(|k| span.error(k, "invalid stencil result"))?;
    Ok(Bound::new(Value::Array(result)))
}

fn power(
    f: &Function,
    operand: &Operand,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    let mut value = right.clone();
    match operand {
        Operand::Array(count) => {
            if !count.is_scalar() { return Err(span.error(ErrorKind::Rank, "power count must be scalar")); }
            let n = count
                .as_number()
                .ok_or_else(|| span.error(ErrorKind::Domain, "power count must be numeric"))?
                .integer()
                .map_err(|k| span.error(k, "power count must be integral"))?;
            let inverse;
            let f = if n < 0 {
                inverse = f.inverse(span)?;
                &inverse
            } else { f };
            let n = n.unsigned_abs();
            for i in 0..n {
                let result = f.call(left, &value, span, session, output)?;
                if i == n - 1 { return Ok(result); }
                value = result.array(span)?;
            }
        }
        Operand::Function(test) => loop {
            let next = f.call_array(left, &value, span, session, output)?;
            let done = test.call_array(Some(&next), &value, span, session, output)?;
            if !done.is_scalar() { return Err(span.error(ErrorKind::Rank, "power predicate must return a scalar")); }
            let done = done
                .as_number()
                .ok_or_else(|| span.error(ErrorKind::Domain, "power predicate must return a Boolean"))?
                .boolean()
                .map_err(|message| span.error(ErrorKind::Domain, message))?;
            value = next;
            if done { break; }
        },
        Operand::Hybrid(_) => unreachable!(),
    }
    Ok(Bound::new(Value::Array(value)))
}

fn inverse(f: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let array = match f.node.as_ref() {
        Primitive(p) => crate::primitive::inverse(*p, left.map(|a| (a, true)), right, None, &session.execution.at(span)),
        Inverse(g) => return g.call(left, right, span, session, output),
        Modified(Each, Operand::Function(g)) => return each(&g.inverse(span)?, left, right, span, session, output),
        Modified(Commute, Operand::Function(g)) => {
            let Primitive(p) = g.node.as_ref() else { return Err(span.error(ErrorKind::Domain, "this commute has no known inverse")); };
            if let Some(x) = left { crate::primitive::inverse(*p, Some((x, false)), right, None, &session.execution.at(span)) } else {
                use crate::number::{Arithmetic, Math};
                match p {
                    crate::primitive::Primitive::Arithmetic(Arithmetic::Plus) => crate::primitive::Primitive::Arithmetic(Arithmetic::Divide).call(
                        Some(right),
                        &Array::scalar(crate::Number::from_integer(2)).unwrap(),
                        &session.execution.at(span),
                    ),
                    crate::primitive::Primitive::Arithmetic(Arithmetic::Times) => {
                        crate::primitive::Primitive::Math(Math::Power).call(Some(right), &Array::scalar(0.5).unwrap(), &session.execution.at(span))
                    }
                    _ => Err(span.error(ErrorKind::Domain, "this commute has no known inverse")),
                }
            }
        }
        Composed(Compose, [Operand::Array(a), Operand::Function(g)]) => return g.inverse(span)?.call(Some(a), right, span, session, output),
        Composed(Compose, [Operand::Function(g), Operand::Array(a)]) => {
            let Primitive(p) = g.node.as_ref() else { return Err(span.error(ErrorKind::Domain, "this right-bound function has no known inverse")); };
            crate::primitive::inverse(*p, Some((a, false)), right, None, &session.execution.at(span))
        }
        Composed(op @ (Compose | Rank | Over), [Operand::Function(g), Operand::Function(h)]) => {
            let transformed = if matches!(op, Over) { left.map(|x| h.call_array(None, x, span, session, output)).transpose()? } else { left.cloned() };
            let y = g.inverse(span)?.call_array(if matches!(op, Rank) { None } else { transformed.as_ref() }, right, span, session, output)?;
            return h.inverse(span)?.call(if matches!(op, Rank) { left } else { None }, &y, span, session, output);
        }
        Composed(Power, [Operand::Function(g), Operand::Array(count)]) => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, &session.execution.at(span))?;
            return power(g, &Operand::Array(count), left, right, span, session, output);
        }
        Composed(Rank, [Operand::Function(g), Operand::Array(ranks)]) => return rank(&g.inverse(span)?, ranks, left, right, span, session, output),
        Fold(g, h) if h.scan && left.is_none() => inverse_scan(g, *h, right, &session.execution.at(span)),
        Axis(g, axis) => match g.node.as_ref() {
            Primitive(p @ crate::primitive::Primitive::Reverse(_)) => {
                crate::primitive::inverse(*p, left.map(|a| (a, true)), right, Some(crate::primitive::single_axis(axis, span)?), &session.execution.at(span))
            }
            Fold(g, h) if h.scan && left.is_none() => {
                inverse_scan(g, Hybrid { axis: Some(crate::primitive::single_axis(axis, span)?), ..*h }, right, &session.execution.at(span))
            }
            _ => Err(span.error(ErrorKind::Domain, "this axis-qualified function has no known inverse")),
        },
        _ => Err(span.error(ErrorKind::Domain, "this function has no known inverse")),
    }?;
    Ok(Bound::new(Value::Array(array)))
}

fn inverse_scan(f: &Function, h: Hybrid, right: &Array, span: &crate::execution::Context<'_>) -> Result<Array, Error> {
    use crate::{number::Arithmetic::*, primitive::Comparison};
    let op = match f.node.as_ref() {
        FunctionNode::Primitive(Primitive::Arithmetic(Plus)) => Primitive::Arithmetic(Minus),
        FunctionNode::Primitive(Primitive::Arithmetic(Times)) => Primitive::Arithmetic(Divide),
        FunctionNode::Primitive(Primitive::Compare(Comparison::NotEqual)) => Primitive::Compare(Comparison::NotEqual),
        _ => return Err(span.error(ErrorKind::Domain, "this scan has no known inverse")),
    };
    if right.is_scalar() && h.axis.is_none() { return Ok(right.clone()); }
    let axis = h.axis.unwrap_or(if h.first { 0 } else { right.shape().len().saturating_sub(1) });
    let axis = crate::array::Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid inverse scan axis"))?;
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in 1..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                data[offset] = Element::Nested(op.call(Some(&right.at(offset).as_array()), &right.at(axis.offset(i, j - 1, k)).as_array(), span)?);
            }
        }
    }
    Array::from_parts(right.shape().to_vec(), data, right.prototype().clone()).map_err(|k| span.error(k, "invalid inverse scan"))
}

fn key(f: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let keys = left.unwrap_or(right);
    if keys.is_scalar() || right.is_scalar() { return Err(span.error(ErrorKind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(span.error(ErrorKind::Length, "key arguments must have equal tallies")); }
    let values = if left.is_none() {
        Primitive::Iota.call(None, &Array::scalar(crate::Number::from_integer(right.shape()[0] as i64)).unwrap(), &session.execution.at(span))?
    } else { right.clone() };
    let cells = keys.cells(keys.shape().len() - 1).map_err(|k| span.error(k, "invalid key cells"))?;
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        let mut found = None;
        for (g, (representative, _)) in groups.iter().enumerate() {
            if crate::primitive::array_match(cell, &cells[*representative], &session.execution.at(span))? {
                found = Some(g);
                break;
            }
        }
        if let Some(g) = found { groups[g].1.push(i); } else { groups.push((i, vec![i])); }
    }
    let count = groups.len();
    if count == 0 { groups.push((0, Vec::new())); }
    let width = generated_len(&values.shape()[1..]).map_err(|k| span.error(k, "key cell is too large"))?;
    let mut results = Vec::with_capacity(groups.len());
    for (representative, indices) in groups {
        let x = if count == 0 {
            let shape = keys.shape()[1..].to_vec();
            let size = generated_len(&shape).map_err(|k| span.error(k, "key prototype is too large"))?;
            Array::from_parts(shape, vec![keys.prototype().clone(); size], keys.prototype().clone()).map_err(|k| span.error(k, "invalid key prototype"))?
        } else { cells[representative].clone() };
        let shape = [&[indices.len()], &values.shape()[1..]].concat();
        let data = indices.into_iter().flat_map(|i| values.items(i * width..(i + 1) * width)).collect();
        let y = Array::from_parts(shape, data, values.prototype().clone()).map_err(|k| span.error(k, "invalid key group"))?;
        results.push(f.call_array(Some(&x), &y, span, session, output)?);
    }
    let result = Array::assemble(&[count], if count == 0 { &[] } else { &results }, &results[0]).map_err(|k| span.error(k, "invalid key result"))?;
    Ok(Bound::new(Value::Array(result)))
}

fn outer(operand: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "outer product needs a left argument"))?;
    let shape = [left.shape(), right.shape()].concat();
    let count = generated_len(&shape).map_err(|k| span.error(k, "outer product is too large"))?;
    let mut data = Vec::with_capacity(count.max(1));
    let mut missing = false;
    for i in 0..count.max(1) {
        let (x, y) = if count == 0 {
            (left.elements().next().unwrap_or_else(|| left.prototype().clone()), right.elements().next().unwrap_or_else(|| right.prototype().clone()))
        } else { (left.at(i / right.len()), right.at(i % right.len())) };
        match operand.call(Some(&x.as_array()), &y.as_array(), span, session, output)?.value {
            Value::Array(a) => data.push(Element::Nested(a)),
            Value::NoResult => missing = true,
            _ => return Err(span.error(ErrorKind::Syntax, "outer product operand must return an array or no result")),
        }
    }
    let value = if missing { Value::NoResult } else {
        let result = if count == 0 { Array::empty(shape, data[0].prototype()) } else { Array::new(shape, data) };
        Value::Array(result.map_err(|k| span.error(k, "invalid outer product result"))?)
    };
    Ok(Bound::new(value))
}

fn inner(
    f: &Function,
    g: &Function,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "inner product needs a left argument"))?;
    let nx = left.shape().last().copied().unwrap_or(1);
    let ny = right.shape().first().copied().unwrap_or(1);
    if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(span.error(ErrorKind::Length, "product contraction lengths must agree")); }
    let n = if left.is_singleton() { ny } else { nx };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = &right.shape()[usize::from(!right.is_scalar())..];
    let shape = [xf, yf].concat();
    let size = generated_len(&shape).map_err(|k| span.error(k, "inner product is too large"))?;
    let rows = generated_len(xf).map_err(|k| span.error(k, "invalid product frame"))?;
    let cols = generated_len(yf).map_err(|k| span.error(k, "invalid product frame"))?;
    generated_len(&[n.max(1), cols.max(1)]).map_err(|k| span.error(k, "product contraction is too large"))?;
    let mut results = Vec::with_capacity(size.max(1));
    let item = |a: &Array, offset| if n == 0 || a.is_empty() { a.prototype().clone() } else { a.at(if a.is_singleton() { 0 } else { offset }) }.as_array();
    for i in 0..rows.max(1) {
        let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
        for k in 0..n.max(1) {
            for (j, column) in columns.iter_mut().enumerate() {
                column.push(Element::Nested(g.call_array(Some(&item(left, i * nx + k)), &item(right, k * cols + j), span, session, output)?));
            }
        }
        for column in columns {
            let paired = if n == 0 { Array::empty(vec![0], column[0].prototype()) } else { Array::new(vec![n], column) }
                .map_err(|k| span.error(k, "invalid product cell"))?;
            results.push(Element::Nested(fold(f, Hybrid { scan: false, first: false, axis: None }, None, &paired, span, session, output)?));
        }
    }
    let result = if size == 0 { Array::empty(shape, results[0].prototype()) } else { Array::new(shape, results) };
    Ok(Bound::new(Value::Array(result.map_err(|k| span.error(k, "invalid inner product result"))?)))
}

fn rank(
    operand: &Function,
    ranks: &Array,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    if ranks.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "rank operand must be a scalar or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(span.error(ErrorKind::Length, "rank operand needs one to three items")); }
    let ranks = ranks
        .elements()
        .map(|e| match e {
            Element::Number(n) => n.integer().map_err(|k| span.error(k, "cell ranks must be integers")),
            _ => Err(span.error(ErrorKind::Domain, "cell ranks must be numeric")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (p, q, r) = match ranks.as_slice() {
        [r] => (*r, *r, *r),
        [q, r] => (*r, *q, *r),
        [p, q, r] => (*p, *q, *r),
        _ => unreachable!(),
    };
    let cell_rank = |a: &Array, k: isize| if k < 0 { a.shape().len().saturating_sub(k.unsigned_abs()) } else { a.shape().len().min(k as usize) };
    let yr = cell_rank(right, if left.is_some() { r } else { p });
    let xr = left.map(|a| cell_rank(a, q)).unwrap_or(0);
    let yf = &right.shape()[..right.shape().len() - yr];
    let xf = left.map(|a| &a.shape()[..a.shape().len() - xr]).unwrap_or(&[]);
    let agreement = Agreement::new(xf, yf).map_err(|k| span.error(k, "rank frames do not agree"))?;
    let frame = &agreement.shape;
    let count = agreement.len;
    let cells = |a: &Array, r: usize| -> Result<Vec<Array>, Error> {
        if count != 0 { return a.cells(r).map_err(|k| span.error(k, "invalid rank cells")); }
        let shape = a.shape()[a.shape().len() - r..].to_vec();
        let len = generated_len(&shape).map_err(|k| span.error(k, "rank prototype is too large"))?;
        Array::from_parts(shape, vec![a.prototype().clone(); len], a.prototype().clone()).map(|a| vec![a]).map_err(|k| span.error(k, "invalid rank prototype"))
    };
    let ys = cells(right, yr)?;
    let xs = left.map(|a| cells(a, xr)).transpose()?;
    let mut results = Vec::with_capacity(count.max(1));
    for i in 0..count.max(1) {
        let x = xs.as_ref().map(|v| &v[agreement.left.index(i)]);
        results.push(operand.call_array(x, &ys[agreement.right.index(i)], span, session, output)?);
    }
    let result = Array::assemble(frame, if count == 0 { &[] } else { &results }, &results[0]).map_err(|k| span.error(k, "invalid rank result"))?;
    Ok(Bound::new(Value::Array(result)))
}

fn each(operand: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let agreement = Agreement::new(left.map_or(&[], Array::shape), right.shape()).map_err(|k| span.error(k, "Each frames do not agree"))?;
    let empty = agreement.len == 0;
    let item = |a: &Array, i: usize| if empty { a.prototype().clone() } else { a.at(i) }.as_array();
    let mut data = Vec::with_capacity(agreement.len.max(1));
    let mut missing = false;
    for i in 0..agreement.len.max(1) {
        let x = left.map(|a| item(a, agreement.left.index(i)));
        let y = item(right, agreement.right.index(i));
        match operand.call(x.as_ref(), &y, span, session, output)?.value {
            Value::Array(a) => data.push(Element::Nested(a)),
            Value::NoResult => missing = true,
            _ => return Err(span.error(ErrorKind::Syntax, "Each operand must return an array or no result")),
        }
    }
    let value = if missing { Value::NoResult } else {
        let result = if empty { Array::empty(agreement.shape, data[0].prototype()) } else { Array::new(agreement.shape, data) };
        Value::Array(result.map_err(|k| span.error(k, "invalid Each result"))?)
    };
    Ok(Bound::new(value))
}

fn identity(operand: &Function, prototype: &Element, span: &crate::execution::Context<'_>) -> Result<Element, Error> {
    use crate::{
        number::{Arithmetic::*, Math::*},
        primitive::Comparison::*,
    };
    if let FunctionNode::Primitive(p @ (Primitive::Ravel | Primitive::CatenateFirst | Primitive::Union)) = operand.node.as_ref() {
        let p = if matches!(p, Primitive::CatenateFirst) { Primitive::Replicate(true) } else { Primitive::Replicate(false) };
        return p.call(Some(&Array::scalar(0.).unwrap()), &prototype.as_array(), span).map(Element::Nested);
    }
    let n = match operand.node.as_ref() {
        FunctionNode::Primitive(
            Primitive::Arithmetic(Plus | Minus) | Primitive::Math(Magnitude | Gcd) | Primitive::Compare(Less | Greater | NotEqual) | Primitive::Reverse(_),
        ) => 0.0,
        FunctionNode::Primitive(
            Primitive::Arithmetic(Times | Divide)
            | Primitive::Math(Power | Factorial | Lcm)
            | Primitive::Compare(Equal | LessEqual | GreaterEqual)
            | Primitive::Replicate(_)
            | Primitive::Expand(_),
        ) => 1.0,
        FunctionNode::Primitive(Primitive::Math(Floor | Ceiling)) => {
            let maximum = matches!(operand.node.as_ref(), FunctionNode::Primitive(Primitive::Math(Floor)));
            if maximum { f64::MAX } else { -f64::MAX }
        }
        _ => return Err(span.error(ErrorKind::Domain, "this function has no reduction identity")),
    };
    match prototype {
        Element::Number(_) | Element::Character(_) if matches!(operand.node.as_ref(), FunctionNode::Primitive(Primitive::Compare(_))) => {
            Ok(Element::Number(crate::Number::from_integer(n as i64)))
        }
        Element::Number(value) => {
            if n.abs() == f64::MAX {
                if value.is_exact() { return Err(span.error(ErrorKind::Domain, "exact min/max has no finite reduction identity")); }
                Ok(Element::Number(n.try_into().unwrap()))
            } else { Ok(Element::Number(value.unit(n as i32))) }
        }
        Element::Character(_) => Ok(Element::Number(n.try_into().unwrap())),
        Element::Nested(a) => {
            let data = a.elements().map(|e| identity(operand, &e, span)).collect::<Result<_, _>>()?;
            let fill = identity(operand, a.prototype(), span)?;
            Array::from_parts(a.shape().to_vec(), data, fill).map(Element::Nested).map_err(|k| span.error(k, "invalid identity"))
        }
    }
}

fn fold(
    operand: &Function,
    hybrid: Hybrid,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    if let Some(width) = left {
        if hybrid.scan { return Err(span.error(ErrorKind::Syntax, "scan has no left argument")); }
        return nwise(operand, hybrid, width, right, span, session, output);
    }
    if right.is_scalar() && hybrid.axis.is_none() { return Ok(right.clone()); }
    let axis = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len().saturating_sub(1) });
    if axis >= right.shape().len() { return Err(span.error(ErrorKind::Domain, "axis is outside array rank")); }
    let traversal = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid fold axis"))?;
    let mut shape = right.shape().to_vec();
    if !hybrid.scan { shape.remove(axis); }
    let size = generated_len(&shape).map_err(|k| span.error(k, "fold result exceeds array limits"))?;
    if size == 0 { return Array::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid empty fold")); }
    if traversal.len == 0 {
        let item = identity(operand, right.prototype(), &session.execution.at(span))?;
        return Array::new(shape, vec![item; size]).map_err(|k| span.error(k, "invalid identity result"));
    }
    if let FunctionNode::Primitive(Primitive::Arithmetic(op)) = operand.node.as_ref() {
        if let Some(data) = right.as_floats() {
            use crate::number::Arithmetic::{Plus, Times};
            match (op, hybrid.scan) {
                (Plus, false) => return float_fold(data, &traversal, shape, false, 0.0, f64::algebraic_add, span),
                (Times, false) => return float_fold(data, &traversal, shape, false, 1.0, f64::algebraic_mul, span),
                (Plus, true) => return float_fold(data, &traversal, shape, true, 0.0, |a, b| a + b, span),
                (Times, true) => return float_fold(data, &traversal, shape, true, 1.0, |a, b| a * b, span),
                _ => (),
            }
        }
        if right.elements().all(|e| matches!(e, Element::Number(_))) {
            return numeric_fold(*op, hybrid.scan, right, &traversal, shape, &session.execution.at(span));
        }
    }
    let mut data = vec![right.prototype().clone(); size];
    for i in 0..traversal.outer {
        for k in 0..traversal.inner {
            let item = |j| right.at(traversal.offset(i, j, k)).as_array();
            let ends = if hybrid.scan { 0..traversal.len } else { traversal.len - 1..traversal.len };
            for end in ends {
                let mut result = item(end);
                for j in (0..end).rev() { result = operand.call_array(Some(&item(j)), &result, span, session, output)?; }
                let index = if hybrid.scan { traversal.offset(i, end, k) } else { i * traversal.inner + k };
                data[index] = Element::Nested(result);
            }
        }
    }
    Array::new(shape, data).map_err(|k| span.error(k, "invalid fold result"))
}

fn float_fold(values: &[f64], axis: &Axis, shape: Vec<usize>, scan: bool, unit: f64, op: impl Fn(f64, f64) -> f64, span: &Span) -> Result<Array, Error> {
    let mut data = vec![unit; generated_len(&shape).map_err(|k| span.error(k, "fold is too large"))?];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            if scan {
                let mut value = values[axis.offset(i, 0, k)];
                data[axis.offset(i, 0, k)] = value;
                for j in 1..axis.len {
                    value = op(value, values[axis.offset(i, j, k)]);
                    data[axis.offset(i, j, k)] = value;
                }
            } else { data[i * axis.inner + k] = (0..axis.len).map(|j| values[axis.offset(i, j, k)]).fold(unit, &op); }
        }
    }
    Array::floats(shape, data).map_err(|k| span.error(k, "fold result is not finite"))
}

fn numeric_fold(
    op: crate::number::Arithmetic,
    scan: bool,
    right: &Array,
    axis: &Axis,
    shape: Vec<usize>,
    span: &crate::execution::Context<'_>,
) -> Result<Array, Error> {
    use crate::number::Arithmetic::{Plus, Times};
    let mut data = vec![right.prototype().clone(); generated_len(&shape).map_err(|k| span.error(k, "fold is too large"))?];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| {
                let Element::Number(n) = right.at(axis.offset(i, j, k)) else { unreachable!() };
                n
            };
            let apply = |x: &crate::Number, y: &crate::Number| {
                span.check()?;
                x.dyad(op, y).map_err(|m| span.error(ErrorKind::Domain, m))
            };
            let cumulative = matches!(op, Plus | Times) && (scan || right.shape().len() == 1);
            let mut previous = item(0);
            for end in if scan { 0..axis.len } else { axis.len - 1..axis.len } {
                let result = if cumulative {
                    let mut result = if scan { previous.clone() } else { item(0) };
                    for j in if scan { end.max(1)..end + 1 } else { 1..end + 1 } { result = apply(&result, &item(j))?; }
                    result
                } else {
                    let mut result = item(end);
                    for j in (0..end).rev() { result = apply(&item(j), &result)?; }
                    result
                };
                data[if scan { axis.offset(i, end, k) } else { i * axis.inner + k }] = Element::Number(result.clone());
                previous = result;
            }
        }
    }
    Array::new(shape, data).map_err(|k| span.error(k, "invalid numeric fold"))
}

fn nwise(
    operand: &Function,
    hybrid: Hybrid,
    width: &Array,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    if !width.is_singleton() { return Err(span.error(ErrorKind::Length, "n-wise reduction needs one width")); }
    let Element::Number(n) = &width.at(0) else { return Err(span.error(ErrorKind::Domain, "reduction width must be an integer")); };
    let n = n.integer().map_err(|k| span.error(k, "reduction width must be an integer"))?;
    if right.is_scalar() { return Err(span.error(ErrorKind::Rank, "n-wise reduction needs a non-scalar array")); }
    let axis = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len() - 1 });
    let source = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid reduction axis"))?;
    let width = n.unsigned_abs();
    let len =
        source.len.checked_add(1).and_then(|v| v.checked_sub(width)).ok_or_else(|| span.error(ErrorKind::Length, "reduction width exceeds axis length + 1"))?;
    let mut shape = right.shape().to_vec();
    shape[axis] = len;
    let size = generated_len(&shape).map_err(|k| span.error(k, "n-wise reduction exceeds array limits"))?;
    if size == 0 { return Array::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid empty reduction")); }
    if width == 0 {
        let item = identity(operand, right.prototype(), &session.execution.at(span))?;
        return Array::new(shape, vec![item; size]).map_err(|k| span.error(k, "invalid identity array"));
    }
    let target = Axis::new(&shape, axis).unwrap();
    let mut data = vec![right.prototype().clone(); size];
    for i in 0..source.outer {
        for k in 0..source.inner {
            for start in 0..len {
                let item = |j| right.at(source.offset(i, start + if n < 0 { width - 1 - j } else { j }, k)).as_array();
                let mut result = item(width - 1);
                for j in (0..width - 1).rev() { result = operand.call_array(Some(&item(j)), &result, span, session, output)?; }
                data[target.offset(i, start, k)] = Element::Nested(result);
            }
        }
    }
    Array::new(shape, data).map_err(|k| span.error(k, "invalid n-wise reduction result"))
}

// A lexical link is an index into active frames, never an owning reference.
// Functions cannot escape: results/array elements are arrays, assignments are local,
// and the public API exports no functions. Revisit this proof before adding an outlet.
#[derive(Clone, Debug)]
struct Closure { definition: Rc<Definition>, environment: Option<usize> }

impl Closure {
    fn text(&self) -> &str {
        let span = &self.definition.span;
        &span.source.text[span.range.clone()]
    }
}
impl Hybrid { fn text(self) -> String { format!("{}{}", self.primitive().glyph(), self.axis.map_or(String::new(), |a| format!("[{}]", a + 1))) } }
struct Frame { names: HashMap<String, Value>, parent: Option<usize> }

#[derive(Clone, Debug)]
enum Operand { Array(Array), Function(Function), Hybrid(Hybrid) }

#[derive(Clone, Debug)]
enum Operator { Defined(Closure), Primitive(OperatorKind), Bound(Box<Operator>, Operand) }

fn operand_dependencies<'a>(operands: impl Iterator<Item = &'a Operand>) -> (usize, Option<usize>) {
    operands.fold((0, None), |(depth, environment), op| match op {
        Operand::Function(f) => (depth.max(f.depth), environment.max(f.environment)),
        _ => (depth, environment),
    })
}

impl Operator {
    fn is_dyadic(&self) -> bool {
        match self {
            Self::Defined(c) => c.definition.kind == DefinitionKind::DyadicOperator,
            Self::Primitive(op) => !matches!(op, OperatorKind::Each | OperatorKind::Commute | OperatorKind::Outer | OperatorKind::Key),
            Self::Bound(..) => false,
        }
    }

    fn derive(self, operand: Operand, span: &Span) -> Result<Function, Error> {
        let node = match self {
            Self::Defined(c) => FunctionNode::Derived(c, operand, None),
            Self::Primitive(op) => {
                let operand = operand.normalize(span)?;
                if !matches!(op, OperatorKind::Commute) && !matches!(operand, Operand::Function(_)) {
                    return Err(span.error(ErrorKind::Domain, "operator needs a function operand"));
                }
                FunctionNode::Modified(op, operand)
            }
            Self::Bound(op, right) => match *op {
                Self::Defined(c) => FunctionNode::Derived(c, operand, Some(right)),
                Self::Primitive(op) => FunctionNode::Composed(op, [operand.normalize(span)?, right.normalize(span)?]),
                Self::Bound(..) => unreachable!(),
            },
        };
        Function::new(node, span)
    }
}

impl Operand {
    fn text(&self, budget: &mut usize) -> String {
        match self { Self::Array(a) => a.to_string(), Self::Function(f) => f.text(budget), Self::Hybrid(h) => h.text() }
    }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        match self { Self::Function(f) => f.tree(budget), _ => crate::display::Tree::leaf(self.text(budget)) }
    }
    fn normalize(self, span: &Span) -> Result<Self, Error> {
        if let Self::Hybrid(h) = self {
            let f = Function::primitive(h.primitive());
            Ok(Self::Function(match h.axis { Some(axis) => Function::new(FunctionNode::Axis(f, crate::primitive::axis_value(axis)), span)?, None => f }))
        } else { Ok(self) }
    }

    fn from_value(value: Value) -> Self {
        match value {
            Value::Array(a) => Self::Array(a),
            Value::Function(f) => Self::Function(f),
            Value::Hybrid(h) => Self::Hybrid(h),
            _ => unreachable!(),
        }
    }
    fn value(&self) -> Value {
        match self { Self::Array(a) => Value::Array(a.clone()), Self::Function(f) => Value::Function(f.clone()), Self::Hybrid(h) => Value::Hybrid(*h) }
    }
}

#[derive(Clone, Debug)]
enum Value {
    NoResult,
    Array(Array),
    Function(Function),
    Operator(Operator),
    Hybrid(Hybrid),
}
struct Bound { value: Value, shy: bool, assignment: bool }
struct Application {
    function: Function,
    left: Option<Array>,
    right: Array,
    span: Span,
    unshy: bool,
    selected: bool,
}
enum Step { Done(Bound), Tail(Application) }

impl Bound {
    fn new(value: Value) -> Self { Self { value, shy: false, assignment: false } }
    fn array(self, span: &Span) -> Result<Array, Error> {
        match self.value {
            Value::Array(a) => Ok(a),
            Value::NoResult => Err(span.error(ErrorKind::Value, "expression produced no value")),
            _ => Err(span.error(ErrorKind::Syntax, "expression must produce an array")),
        }
    }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value { Value::Array(_) | Value::NoResult => Ok(self), _ => Err(span.error(ErrorKind::Syntax, "dfn results must be arrays")) }
    }
}

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation { pub value: Option<Array>, pub output: Vec<String>, pub error: Option<Error> }

#[derive(Default)]
pub struct Session {
    execution: crate::execution::Execution,
    pub(crate) display: crate::display::Settings,
    names: HashMap<String, Value>,
    frames: Vec<Frame>,
    current: Option<usize>,
    depth: usize,
    #[cfg(test)]
    peak_frames: usize,
}

impl Session {
    pub fn new() -> Self { Self::default() }
    pub fn set(&mut self, name: &str, value: Array) -> Result<(), ErrorKind> {
        let ParseStatus::Complete(parsed) = crate::parse(Source::new("<binding>", name)) else { return Err(ErrorKind::Syntax); };
        if parsed.statements.len() != 1 || parsed.statements[0].nodes.len() != 1 { return Err(ErrorKind::Syntax); }
        let NodeKind::Name(parsed_name) = &parsed.statements[0].nodes[0].kind else { return Err(ErrorKind::Syntax); };
        if parsed_name != name || matches!(name, "⍺" | "⍵" | "⍺⍺" | "⍵⍵" | "∇" | "∇∇") { return Err(ErrorKind::Syntax); }
        self.names.insert(name.to_owned(), Value::Array(value));
        Ok(())
    }
    pub fn eval(&mut self, code: &str) -> Evaluation { self.eval_source(Source::new("<input>", code)) }
    pub fn eval_with(&mut self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.execution.begin(options);
        self.evaluate_source(Source::new("<input>", code))
    }
    pub fn eval_timeout(&mut self, code: &str, timeout: std::time::Duration) -> Evaluation {
        self.eval_with(code, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() })
    }
    pub fn eval_source(&mut self, source: std::rc::Rc<Source>) -> Evaluation {
        self.execution.begin(crate::EvalOptions::default());
        self.evaluate_source(source)
    }
    fn evaluate_source(&mut self, source: std::rc::Rc<Source>) -> Evaluation {
        if source.text.trim_start().starts_with(']') { return self.command(source); }
        match crate::parse(source) {
            ParseStatus::Complete(parsed) => self.eval_display(&parsed, false),
            ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => Evaluation { error: Some(e), ..Evaluation::default() },
        }
    }
    pub fn eval_parsed(&mut self, parsed: &Parsed) -> Evaluation {
        self.execution.begin(crate::EvalOptions::default());
        self.eval_display(parsed, false)
    }
    fn eval_display(&mut self, parsed: &Parsed, diagram: bool) -> Evaluation {
        let mut result = Evaluation::default();
        for (i, statement) in parsed.statements.iter().enumerate() {
            let nodes = &statement.nodes;
            match self.bind(nodes, &mut result.output) {
                Ok(bound) => {
                    result.value = match bound.value {
                        Value::NoResult => None,
                        Value::Array(a) => {
                            if diagram && i + 1 == parsed.statements.len() { result.output.push(crate::display::diagram(&a)); }
                            else if !bound.shy { result.output.push(self.display.array(&a, false)); }
                            Some(a)
                        }
                        _ if bound.shy => None,
                        Value::Function(f) => {
                            result.output.push(if self.display.enabled && self.display.trees { f.tree(&mut 1000).render() } else { f.text(&mut 1000) });
                            None
                        }
                        Value::Hybrid(h) => {
                            result.output.push(h.text());
                            None
                        }
                        _ => {
                            result.value = None;
                            result.error = Some(nodes[0].span.error(ErrorKind::Syntax, "a function needs a right argument"));
                            return result;
                        }
                    };
                }
                Err(e) => {
                    result.value = None;
                    result.error = Some(e);
                    break;
                }
            }
        }
        result
    }
    fn command(&mut self, source: Rc<Source>) -> Evaluation {
        let code = source.text.trim();
        let (command, args) = code.split_once(char::is_whitespace).unwrap_or((code, ""));
        if matches!(command.to_ascii_lowercase().as_str(), "]box" | "]boxing") {
            return match self.display.configure(args) {
                Ok(text) => Evaluation { output: vec![text], ..Evaluation::default() },
                Err(message) => {
                    Evaluation { error: Some(Span { range: 0..source.text.len(), source }.error(ErrorKind::Domain, message)), ..Evaluation::default() }
                }
            };
        }
        if command.eq_ignore_ascii_case("]display") {
            return match crate::parse(Source::new("<display>", args)) {
                ParseStatus::Complete(parsed) => self.eval_display(&parsed, true),
                ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => Evaluation { error: Some(e), ..Evaluation::default() },
            };
        }
        Evaluation { error: Some(Span { range: 0..source.text.len(), source }.error(ErrorKind::Syntax, "unknown user command")), ..Evaluation::default() }
    }
    fn bind(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Bound, Error> {
        self.execution.check(&nodes[0].span)?;
        if self.depth == MAX_CALL_DEPTH { return Err(nodes[0].span.error(ErrorKind::Limit, "evaluation depth exceeds 64")); }
        self.depth += 1;
        let result = self.bind_expression(nodes, output);
        self.depth -= 1;
        result
    }

    fn execute(&mut self, left: Option<&Array>, right: &Array, span: &Span, output: &mut Vec<String>) -> Result<Bound, Error> {
        if let Some(x) = left {
            if !x.is_empty() || x.shape().len() != 1 || !matches!(x.prototype(), Element::Character(_)) {
                return Err(span.error(ErrorKind::Unsupported, "execute supports only the current namespace (empty character left argument)"));
            }
        }
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "execute needs a character scalar or vector")); }
        if !matches!(right.prototype(), Element::Character(_)) { return Err(span.error(ErrorKind::Domain, "execute needs characters")); }
        let code = right
            .elements()
            .map(|e| match e { Element::Character(c) => Ok(c), _ => Err(span.error(ErrorKind::Domain, "execute needs characters")) })
            .collect::<Result<String, _>>()?;
        let parsed = match crate::parse(Source::new("<execute>", code)) {
            ParseStatus::Complete(p) => p,
            ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => return Err(e),
        };
        let mut result = Bound::new(Value::NoResult);
        for statement in &parsed.statements {
            if let Value::Array(a) = &result.value { if !result.shy { output.push(self.display.array(a, self.current.is_some())); } }
            result = self.bind(&statement.nodes, output).map_err(|mut e| {
                e.calls.push(span.clone());
                e
            })?;
            if !matches!(result.value, Value::Array(_) | Value::NoResult) {
                if !result.shy { return Err(span.error(ErrorKind::Syntax, "execute cannot return a function")); }
                result.value = Value::NoResult;
            }
        }
        Ok(result)
    }

    fn bind_expression(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Bound, Error> {
        let Step::Done(result) = Binder::evaluate(nodes, self, output, false)? else { unreachable!() };
        Ok(result)
    }

    fn assignment_names(&self, nodes: &[Node]) -> bool {
        !nodes.is_empty()
            && nodes.iter().all(|n| match &n.kind {
                NodeKind::Name(name) => {
                    self.current.is_some() || !matches!(self.lookup(name), Some(Value::Function(_) | Value::Hybrid(_) | Value::Operator(_)))
                }
                NodeKind::Group(inner) => self.assignment_names(inner),
                _ => false,
            })
    }

    fn assignment_operand(&self, nodes: &[Node], end: usize, depth: usize) -> Result<(usize, Category), Error> {
        use Category::*;
        if end == 0 { return Err(nodes[0].span.error(ErrorKind::Syntax, "missing assignment operand")); }
        if depth == 128 { return Err(nodes[end - 1].span.error(ErrorKind::Limit, "assignment operand nesting exceeds 128")); }
        let mut start = end - 1;
        let mut category = match &nodes[start].kind {
            NodeKind::Function(_) => Function,
            NodeKind::Hybrid(_) => Hybrid,
            NodeKind::Operator(op) => {
                if self::Operator::Primitive(*op).is_dyadic() { DyadicOperator } else { Operator }
            }
            NodeKind::Name(name) => match self.lookup(name) {
                Some(Value::Function(_)) => Function,
                Some(Value::Hybrid(_)) => Hybrid,
                Some(Value::Operator(op)) => {
                    if op.is_dyadic() { DyadicOperator } else { Operator }
                }
                _ => Array,
            },
            NodeKind::Dfn(d) => match d.kind {
                DefinitionKind::Function => Function,
                DefinitionKind::MonadicOperator => Operator,
                DefinitionKind::DyadicOperator => DyadicOperator,
            },
            NodeKind::Group(inner) => {
                if inner.is_empty() { return Err(nodes[start].span.error(ErrorKind::Syntax, "empty assignment operand")); }
                self.assignment_operand(inner, inner.len(), depth + 1)?.1
            }
            NodeKind::Selection(_) => {
                let (i, c) = self.assignment_operand(nodes, start, depth + 1)?;
                start = i;
                c
            }
            _ => Array,
        };
        if matches!(category, Operator) {
            start = self.assignment_operand(nodes, start, depth + 1)?.0;
            category = Function;
        }
        else if matches!(category, Hybrid) && start > 0 {
            let (i, c) = self.assignment_operand(nodes, start, depth + 1)?;
            if matches!(c, Function | Hybrid) {
                start = i;
                category = Function;
            }
        }
        if start > 0 {
            let dyadic = match &nodes[start - 1].kind {
                NodeKind::Operator(op) => self::Operator::Primitive(*op).is_dyadic(),
                NodeKind::Name(name) => matches!(self.lookup(name), Some(Value::Operator(op)) if op.is_dyadic()),
                _ => false,
            };
            if dyadic {
                start = self.assignment_operand(nodes, start - 1, depth + 1)?.0;
                category = Function;
            }
        }
        Ok((start, category))
    }

    fn assignment_start(&self, nodes: &[Node]) -> Result<usize, Error> {
        let end = nodes.len();
        let (operand, category) = self.assignment_operand(nodes, end, 0)?;
        let names = self.assignment_names(&nodes[operand..]);
        let target_end = if operand > 0 && !names && matches!(category, Category::Function | Category::Hybrid) { operand } else { end };
        let mut start = target_end - 1;
        while matches!(nodes[start].kind, NodeKind::Selection(_)) && start > 0 { start -= 1; }
        if self.assignment_names(&nodes[start..target_end]) { while start > 0 && self.assignment_names(&nodes[start - 1..start]) { start -= 1; } }
        Ok(start)
    }

    fn store(&mut self, name: &str, value: Value, span: &Span) -> Result<(), Error> {
        if matches!(name, "⍺" | "⍵" | "⍺⍺" | "⍵⍵" | "∇" | "∇∇") {
            return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here"));
        }
        let names = match self.current { Some(i) => &mut self.frames[i].names, None => &mut self.names };
        names.insert(name.to_owned(), value);
        Ok(())
    }

    fn indices(&mut self, parts: &[Vec<Node>], output: &mut Vec<String>) -> Result<Vec<Option<Array>>, Error> {
        let mut values = Vec::new();
        for nodes in parts.iter().rev() { values.push(if nodes.is_empty() { None } else { Some(self.array_result(nodes, output)?) }); }
        values.reverse();
        Ok(values)
    }

    fn assign(&mut self, target: &[Node], value: &Value, output: &mut Vec<String>) -> Result<(), Error> {
        let span = &target[0].span;
        if let [target] = target {
            match &target.kind {
                NodeKind::Name(name) => {
                    return self.store(name, value.clone(), span);
                }
                NodeKind::Output => {
                    return match value {
                        Value::Array(a) => {
                            output.push(self.display.array(a, self.current.is_some()));
                            Ok(())
                        }
                        _ => return Err(target.span.error(ErrorKind::Domain, "output requires an array")),
                    }
                }
                NodeKind::Group(nodes) => {
                    if self.assignment_names(nodes) { return self.assign(nodes, value, output); }
                    return self.assign_selected(nodes, None, value, output);
                }
                _ => (),
            }
        }
        let Value::Array(right) = value else { return Err(span.error(ErrorKind::Domain, "multiple or modified assignment needs an array")); };
        if self.assignment_names(target) {
            if !right.is_singleton() && (right.shape().len() != 1 || right.len() != target.len()) {
                return Err(span.error(ErrorKind::Length, "strand assignment needs one item per target"));
            }
            for (i, node) in target.iter().enumerate().rev() {
                self.assign(std::slice::from_ref(node), &Value::Array(right.at(if right.is_singleton() { 0 } else { i }).as_array()), output)?;
            }
            return Ok(());
        }
        let names = self.assignment_operand(target, target.len(), 0)?.0;
        if self.assignment_names(&target[..names]) {
            let modifier = self.modifier(&target[names..], output)?.unwrap();
            return self.modify_names(&target[..names], right, &modifier, output);
        }
        if let NodeKind::Group(nodes) = &target[0].kind {
            let modifier = self.modifier(&target[1..], output)?;
            return self.assign_selected(nodes, modifier, value, output);
        }
        let NodeKind::Name(name) = &target[0].kind else { return Err(span.error(ErrorKind::Syntax, "assignment needs a name or selection")); };
        let end = 1 + target[1..].iter().take_while(|n| matches!(n.kind, NodeKind::Selection(_))).count();
        let modifier = self.modifier(&target[end..], output)?;
        let mut parts = Vec::new();
        for node in target[1..end].iter().rev() {
            let NodeKind::Selection(indices) = &node.kind else { unreachable!() };
            parts.push(self.indices(indices, output)?);
        }
        let Some(Value::Array(original)) = self.lookup(name).cloned() else {
            return Err(span.error(ErrorKind::Value, "assignment target must be an existing array"));
        };
        let updated = if parts.is_empty() { modifier.unwrap().call_array(Some(&original), right, span, self, output)? } else {
            let mut current = original.clone();
            let mut selection: Option<crate::primitive::Selection> = None;
            for parts in parts.iter().rev() {
                let mut next = crate::primitive::selection(&current, parts, &self.execution.at(span))?;
                current = next.read(&current, &self.execution.at(span))?;
                if let Some(previous) = selection { for path in &mut next.paths { *path = [previous.paths[path[0]].as_slice(), &path[1..]].concat(); } }
                selection = Some(next);
            }
            let selection = selection.unwrap();
            if let Some(f) = modifier { self.modify_selection(&original, &selection, &f, right, span, output)? } else { selection.write(&original, right, &self.execution.at(span))? }
        };
        self.store(name, Value::Array(updated), span)
    }

    fn modify_names(&mut self, nodes: &[Node], right: &Array, modifier: &Function, output: &mut Vec<String>) -> Result<(), Error> {
        if let [node] = nodes {
            return match &node.kind {
                NodeKind::Group(inner) => self.modify_names(inner, right, modifier, output),
                NodeKind::Name(name) => {
                    let Some(Value::Array(left)) = self.lookup(name).cloned() else {
                        return Err(node.span.error(ErrorKind::Value, "assignment target must be an existing array"));
                    };
                    let updated = modifier.call_array(Some(&left), right, &node.span, self, output)?;
                    self.store(name, Value::Array(updated), &node.span)
                }
                _ => unreachable!(),
            };
        }
        if !right.is_singleton() && (right.shape().len() != 1 || right.len() != nodes.len()) {
            return Err(nodes[0].span.error(ErrorKind::Length, "strand assignment needs one item per target"));
        }
        for (i, node) in nodes.iter().enumerate() {
            self.modify_names(std::slice::from_ref(node), &right.at(if right.is_singleton() { 0 } else { i }).as_array(), modifier, output)?;
        }
        Ok(())
    }

    fn modifier(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Option<Function>, Error> {
        if nodes.is_empty() { return Ok(None); }
        match self.bind(nodes, output)?.value {
            Value::Function(f) => Ok(Some(f)),
            Value::Hybrid(h) => Ok(Some(Function::primitive(h.primitive()))),
            _ => Err(nodes[0].span.error(ErrorKind::Syntax, "modified assignment needs a function")),
        }
    }

    fn assign_selected(&mut self, nodes: &[Node], modifier: Option<Function>, value: &Value, output: &mut Vec<String>) -> Result<(), Error> {
        let Value::Array(right) = value else { return Err(nodes[0].span.error(ErrorKind::Domain, "selective assignment needs an array")); };
        let (name, original, labels, selected) = self.selection_expression(nodes, output)?;
        let span = &nodes[0].span;
        let (selection, values) = labels.replacements(&selected, right, span)?;
        let updated = match modifier {
            Some(f) => self.modify_selection(&original, &selection, &f, &values, span, output)?,
            None => selection.write(&original, &values, &self.execution.at(span))?,
        };
        self.store(&name, Value::Array(updated), span)
    }

    fn selection_expression(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<(String, Array, crate::selection::Labels, Array), Error> {
        let root = nodes
            .iter()
            .rposition(|n| !matches!(n.kind, NodeKind::Selection(_)))
            .ok_or_else(|| nodes[0].span.error(ErrorKind::Syntax, "selection needs a name"))?;
        let span = &nodes[root].span;
        let (name, original, labels, selected) = match &nodes[root].kind {
            NodeKind::Name(name) => {
                let Some(Value::Array(original)) = self.lookup(name).cloned() else {
                    return Err(span.error(ErrorKind::Value, "selection needs an array name"));
                };
                let (labels, selected) = crate::selection::Labels::new(&original, span)?;
                (name.clone(), original, labels, selected)
            }
            NodeKind::Group(inner) => self.selection_expression(inner, output)?,
            _ => return Err(span.error(ErrorKind::Syntax, "selection must end in an array name")),
        };
        let Step::Done(result) = Binder::evaluate_marked(nodes, self, output, false, Some((root, selected)))? else { unreachable!() };
        Ok((name, original, labels, result.array(span)?))
    }

    fn modify_selection(
        &mut self,
        original: &Array,
        selection: &crate::primitive::Selection,
        f: &Function,
        right: &Array,
        span: &Span,
        output: &mut Vec<String>,
    ) -> Result<Array, Error> {
        if !right.is_singleton() && right.shape() != selection.shape {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        if selection.paths.iter().all(|p| p.len() == 1) {
            let mut data: Vec<_> = original.elements().collect();
            for (i, path) in selection.paths.iter().enumerate() {
                let r = right.at(if right.is_singleton() { 0 } else { i }).as_array();
                let result = f.call_array(Some(&data[path[0]].as_array()), &r, span, self, output)?;
                data[path[0]] = Element::Nested(result);
            }
            return Array::from_parts(original.shape().to_vec(), data, original.prototype().clone()).map_err(|k| span.error(k, "invalid modified selection"));
        }
        let mut updated = original.clone();
        for (i, path) in selection.paths.iter().enumerate() {
            let one = crate::primitive::Selection { shape: vec![], paths: vec![path.clone()] };
            let left = one.read(&updated, &self.execution.at(span))?.disclose();
            let r = right.at(if right.is_singleton() { 0 } else { i }).as_array();
            let result = f.call_array(Some(&left), &r, span, self, output)?;
            let boxed = Array::new(vec![], vec![Element::Nested(result)]).map_err(|k| span.error(k, "invalid modified result"))?;
            updated = one.write(&updated, &boxed, &self.execution.at(span))?;
        }
        Ok(updated)
    }

    // Resolving one structural item may execute a group, but never derives an operator
    // or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    fn resolve(&mut self, node: &Node, output: &mut Vec<String>) -> Result<Value, Error> {
        Ok(match &node.kind {
            NodeKind::Literal(a) => Value::Array(a.clone()),
            NodeKind::Function(p) => Value::Function(Function::primitive(*p)),
            NodeKind::Operator(op) => Value::Operator(Operator::Primitive(*op)),
            NodeKind::Hybrid(h) => Value::Hybrid(*h),
            NodeKind::Name(name) => self.lookup(name).cloned().ok_or_else(|| node.span.error(ErrorKind::Value, format!("undefined name: {name}")))?,
            NodeKind::Group(nodes) => self.bind(nodes, output)?.value,
            NodeKind::ArrayLiteral { cells, block } => {
                let arrays = cells.iter().map(|nodes| self.array_result(nodes, output)).collect::<Result<Vec<_>, _>>()?;
                let result = if *block {
                    let arrays =
                        arrays.into_iter().map(|a| if a.is_scalar() { Array::new(vec![1], a.elements().collect()).unwrap() } else { a }).collect::<Vec<_>>();
                    Array::assemble(&[arrays.len()], &arrays, &arrays[0])
                } else { Array::new(vec![arrays.len()], arrays.into_iter().map(Element::Nested).collect()) };
                Value::Array(result.map_err(|k| node.span.error(k, "invalid array literal"))?)
            }
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.current };
                match definition.kind {
                    DefinitionKind::Function => Value::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Value::Operator(Operator::Defined(closure)),
                }
            }
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment or output symbol")),
        })
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        if name == "⍺" { return self.current.and_then(|i| self.frames[i].names.get(name)); }
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].names.get(name) { return Some(value); }
            scope = self.frames[i].parent;
        }
        self.names.get(name)
    }

    fn call_defined(&mut self, function: &Function, left: Option<&Array>, right: &Array, output: &mut Vec<String>) -> Result<Bound, Error> {
        let caller = self.current;
        let base = self.frames.len();
        let (mut function, mut left, mut right) = (function.clone(), left.cloned(), right.clone());
        let mut tail_span = None;
        let mut unshy = false;
        let mut result = loop {
            let (closure, operand, right_operand) = match function.node.as_ref() {
                FunctionNode::Defined(c) => (c, None, None),
                FunctionNode::Derived(c, operand, right) => (c, Some(operand), right.as_ref()),
                _ => unreachable!(),
            };
            if self.frames.len() == MAX_CALL_DEPTH { break Err(closure.definition.span.error(ErrorKind::Limit, "lexical frame depth exceeds 64")); }
            let mut names = HashMap::from([("⍵".into(), Value::Array(right)), ("∇".into(), Value::Function(function.clone()))]);
            if let Some(a) = left { names.insert("⍺".into(), Value::Array(a)); }
            if let Some(f) = operand {
                names.insert("⍺⍺".into(), f.value());
                names.insert("∇∇".into(), Value::Operator(Operator::Defined(closure.clone())));
            }
            if let Some(f) = right_operand { names.insert("⍵⍵".into(), f.value()); }
            self.current = Some(self.frames.len());
            self.frames.push(Frame { names, parent: closure.environment });
            #[cfg(test)]
            { self.peak_frames = self.peak_frames.max(self.frames.len()); }
            match self.run_definition(&closure.definition, output) {
                Ok(Step::Done(bound)) => break Ok(bound),
                Err(error) => break Err(error),
                Ok(Step::Tail(call)) => {
                    // Keep lexical dependencies, not the tail caller's execution frame.
                    let keep = base.max(call.function.environment.map_or(0, |i| i + 1));
                    self.frames.truncate(keep);
                    function = call.function;
                    left = call.left;
                    right = call.right;
                    tail_span = Some(call.span);
                    unshy |= call.unshy;
                }
            }
        };
        if let (Err(error), Some(span)) = (&mut result, tail_span) { error.calls.push(span); }
        if let Ok(bound) = &mut result { if unshy { bound.shy = false; } }
        self.frames.truncate(base);
        self.current = caller;
        result
    }

    fn array_result(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Array, Error> { self.bind(nodes, output)?.array(&nodes[0].span) }

    fn return_expression(&mut self, mut nodes: &[Node], output: &mut Vec<String>, tail: bool) -> Result<Step, Error> {
        let mut unshy = false;
        while let [Node { kind: NodeKind::Group(inner), .. }] = nodes {
            nodes = inner;
            unshy = true;
        }
        let step = if nodes.iter().any(|n| matches!(n.kind, NodeKind::Assign)) { Step::Done(self.bind(nodes, output)?) } else { Binder::evaluate(nodes, self, output, tail)? };
        match step {
            Step::Done(mut bound) => {
                if unshy { bound.shy = false; }
                bound.result(&nodes[0].span).map(Step::Done)
            }
            Step::Tail(mut call) => {
                call.unshy = unshy;
                Ok(Step::Tail(call))
            }
        }
    }

    fn run_definition(&mut self, definition: &Definition, output: &mut Vec<String>) -> Result<Step, Error> {
        // Guards are dynamic call state. A small binding-map checkpoint is sufficient
        // for this slice: values share storage, and restoration also removes new names.
        // This snapshots rollback state, NOT lexical captures (which use live frames).
        let frame = self.current.unwrap();
        let mut handlers = Vec::new();
        let result = (|| {
            for (position, statement) in definition.body.statements.iter().enumerate() {
                let nodes = &statement.nodes;
                if statement.guard.is_none()
                    && matches!(&nodes[0].kind, NodeKind::Name(name) if name == "⍺")
                    && matches!(nodes.get(1).map(|n| &n.kind), Some(NodeKind::Assign))
                {
                    if nodes.len() == 2 { return Err(nodes[1].span.error(ErrorKind::Syntax, "default argument needs a value")); }
                    if !self.frames[frame].names.contains_key("⍺") {
                        let value = self.bind(&nodes[2..], output)?.value;
                        if matches!(value, Value::NoResult) { return Err(nodes[0].span.error(ErrorKind::Value, "default argument requires a value")); }
                        self.frames[frame].names.insert("⍺".into(), value);
                    }
                    continue;
                }
                if let Some((i, error_guard)) = statement.guard {
                    let condition = self.array_result(&nodes[..i], output)?;
                    if error_guard {
                        if condition.shape().len() > 1 { return Err(nodes[i].span.error(ErrorKind::Rank, "error numbers must be a scalar or vector")); }
                        let numbers = condition
                            .elements()
                            .map(|e| match e {
                                Element::Number(n) => n.nonnegative_integer().map_err(|k| nodes[i].span.error(k, "invalid error number")),
                                _ => Err(nodes[i].span.error(ErrorKind::Domain, "error numbers must be numeric")),
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        handlers.push((&nodes[i + 1..], numbers, self.frames[frame].names.clone()));
                    } else {
                        let condition = condition.as_number().ok_or_else(|| nodes[i].span.error(ErrorKind::Domain, "guard requires a Boolean scalar"))?;
                        let condition = condition.nonnegative_integer().map_err(|k| nodes[i].span.error(k, "invalid guard condition"))?;
                        if condition > 1 { return Err(nodes[i].span.error(ErrorKind::Domain, "guard requires 0 or 1")); }
                        if condition == 1 { return self.return_expression(&nodes[i + 1..], output, handlers.is_empty()); }
                    }
                } else {
                    let assignment = nodes.iter().any(|n| matches!(n.kind, NodeKind::Assign));
                    if !assignment || position + 1 == definition.body.statements.len() { return self.return_expression(nodes, output, handlers.is_empty()); }
                    let bound = self.bind(nodes, output)?;
                    if !bound.assignment { return bound.result(&nodes[0].span).map(Step::Done); }
                }
            }
            Ok(Step::Done(Bound::new(Value::NoResult)))
        })();
        let mut result = result;
        while let Err(error) = &result {
            // Unsupported subset features must not turn into plausible successful results.
            let Some(number) = error.kind.number() else { break; };
            let Some((handler, numbers, checkpoint)) = handlers.pop() else { break; };
            if !numbers.contains(&0) && !numbers.contains(&number) { continue; }
            self.frames[frame].names = checkpoint;
            result = self.return_expression(handler, output, false);
        }
        result
    }
}

// Only the binder has unfinished strands, trains, and bound left arguments.
// Names and groups contain completed Values, so grouping never flattens a strand.
#[derive(Clone, Copy)]
enum Category {
    NoResult,
    Array,
    Function,
    Hybrid,
    Operator,
    DyadicOperator,
    Left,
    Selection,
}
enum Term {
    Value(Value),
    Strand(Vec<Element>),
    Train(Vec<Function>),
    Left(Array, Function),
    Selection(Vec<Option<Array>>),
}
struct Entity {
    term: Term,
    span: Span,
    shy: bool,
    selected: bool,
    assignment: bool,
}

impl Entity {
    fn category(&self) -> Category {
        match self.term {
            Term::Value(Value::NoResult) => Category::NoResult,
            Term::Value(Value::Array(_)) | Term::Strand(_) => Category::Array,
            Term::Value(Value::Function(_)) | Term::Train(_) => Category::Function,
            Term::Value(Value::Hybrid(_)) => Category::Hybrid,
            Term::Value(Value::Operator(ref op)) => {
                if op.is_dyadic() { Category::DyadicOperator } else { Category::Operator }
            }
            Term::Left(..) => Category::Left,
            Term::Selection(_) => Category::Selection,
        }
    }
    fn value(self) -> Result<Value, Error> {
        Ok(match self.term {
            Term::Value(v) => v,
            Term::Strand(items) => Value::Array(Array::new(vec![items.len()], items).map_err(|k| self.span.error(k, "invalid strand"))?),
            Term::Train(fs) => Value::Function(self::Function::train(fs, &self.span)?),
            Term::Left(..) => return Err(self.span.error(ErrorKind::Syntax, "a function needs a right argument")),
            Term::Selection(_) => return Err(self.span.error(ErrorKind::Syntax, "index/axis brackets need an array or function to their left")),
        })
    }
    fn function(self) -> Result<Function, Error> {
        let span = self.span.clone();
        match self.value()? {
            Value::Function(f) => Ok(f),
            Value::Hybrid(h) => {
                let f = Function::primitive(h.primitive());
                match h.axis { Some(axis) => Function::new(FunctionNode::Axis(f, crate::primitive::axis_value(axis)), &span), None => Ok(f) }
            }
            _ => unreachable!(),
        }
    }
    fn array(self) -> Result<Array, Error> { match self.value()? { Value::Array(a) => Ok(a), _ => unreachable!() } }
}

// Implemented rows of Dyalog 20's category table, not per-glyph precedence.
fn strength(left: &Entity, right: &Entity) -> u8 {
    use Category::*;
    match (left.category(), right.category()) {
        (Array | Function | Hybrid, Selection) => 4,
        (DyadicOperator, Array | Function | Hybrid) => 5,
        (DyadicOperator, Operator) => 4,
        (Array, Array) => 6,
        (Array | Function | Hybrid, Operator) | (Function | Hybrid | Operator, Hybrid) => 4,
        (Array, Function | Hybrid) | (Selection, Array | Function | Hybrid) => 3,
        (Function | Left, Array) => 2,
        (Function | Hybrid | Left, Function) => 1,
        _ => 0,
    }
}

struct Binder { stack: Vec<Entity> }
impl Binder {
    fn evaluate(nodes: &[Node], session: &mut Session, output: &mut Vec<String>, tail: bool) -> Result<Step, Error> {
        Self::evaluate_marked(nodes, session, output, tail, None)
    }
    fn evaluate_marked(nodes: &[Node], session: &mut Session, output: &mut Vec<String>, tail: bool, marked: Option<(usize, Array)>) -> Result<Step, Error> {
        let mut binder = Self { stack: Vec::new() };
        let mut cursor = nodes.len();
        let mut assignment = None;
        let mut pending = Vec::new();
        loop {
            session.execution.check(&nodes[0].span)?;
            let next = if let Some(entity) = pending.pop() { Some(entity) } else if assignment.is_none() && cursor > 0 {
                cursor -= 1;
                let i = cursor;
                let node = &nodes[i];
                if matches!(node.kind, NodeKind::Assign) {
                    assignment = Some(i);
                    None
                } else {
                    let selected = marked.as_ref().is_some_and(|(index, _)| *index == i);
                    let term = if selected {
                        Term::Value(Value::Array(marked.as_ref().unwrap().1.clone()))
                    } else if let NodeKind::Selection(parts) = &node.kind { Term::Selection(session.indices(parts, output)?) } else { Term::Value(session.resolve(node, output)?) };
                    Some(Entity { term, span: node.span.clone(), shy: false, selected, assignment: false })
                }
            } else { None };
            let n = binder.stack.len();
            if let Some(left) = next {
                if n < 2 || strength(&left, &binder.stack[n - 1]) >= strength(&binder.stack[n - 1], &binder.stack[n - 2]) {
                    binder.stack.push(left);
                    continue;
                }
                pending.push(left);
            }
            else if n < 2 {
                if let Some(i) = assignment.take() {
                    let begin = nodes[..i].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |j| j + 1);
                    if begin == i || n == 0 { return Err(nodes[i].span.error(ErrorKind::Syntax, "assignment needs a target and value")); }
                    let entity = binder.stack.pop().unwrap();
                    let value = entity.value()?;
                    if matches!(value, Value::NoResult) { return Err(nodes[i].span.error(ErrorKind::Value, "assignment requires a value")); }
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    session.assign(&nodes[cursor..i], &value, output)?;
                    pending.push(Entity { term: Term::Value(value), span: nodes[i].span.clone(), shy: true, selected: false, assignment: true });
                    continue;
                }
                break;
            }
            if let Some(call) = binder.reduce(&session.execution)? {
                if tail
                    && cursor == 0
                    && assignment.is_none()
                    && pending.is_empty()
                    && binder.stack.is_empty()
                    && matches!(call.function.node.as_ref(), FunctionNode::Defined(_) | FunctionNode::Derived(..))
                { return Ok(Step::Tail(call)); }
                if call.selected && !call.function.selects(call.left.is_some()) {
                    return Err(call.span.error(ErrorKind::Domain, "function is not valid for selective assignment"));
                }
                let bound = call.function.call(call.left.as_ref(), &call.right, &call.span, session, output)?;
                binder.stack.push(Entity { term: Term::Value(bound.value), span: call.span, shy: bound.shy, selected: call.selected, assignment: false });
            }
            // Category changes must rebind against the remaining right context.
            pending.push(binder.stack.pop().unwrap());
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        let shy = entity.shy;
        let assignment = entity.assignment;
        Ok(Step::Done(Bound { value: entity.value()?, shy, assignment }))
    }

    fn reduce(&mut self, execution: &crate::execution::Execution) -> Result<Option<Application>, Error> {
        use Category::*;
        let left = self.stack.pop().unwrap();
        let right = self.stack.pop().unwrap();
        let span = left.span.clone();
        let right_span = right.span.clone();
        let selected = left.selected || right.selected;
        if selected && !matches!((left.category(), right.category()), (Array, Selection) | (Function | Left, Array)) {
            return Err(span.error(ErrorKind::Syntax, "invalid selective-assignment expression"));
        }
        let term = match (left.category(), right.category()) {
            (NoResult, _) | (_, NoResult) => return Err(span.error(ErrorKind::Value, "expression produced no value")),
            (Array | Function | Hybrid, Selection) => {
                let Term::Selection(parts) = right.term else { unreachable!() };
                if matches!(left.category(), Array) {
                    Term::Value(Value::Array(crate::primitive::select(&left.array()?, &parts, &execution.at(&right_span))?))
                } else {
                    let [Some(axis)] = parts.as_slice() else { return Err(right_span.error(ErrorKind::Syntax, "one axis expression is required")); };
                    if matches!(left.category(), Hybrid) {
                        let axis = crate::primitive::single_axis(axis, &right_span)?;
                        let Value::Hybrid(h) = left.value()? else { unreachable!() };
                        Term::Value(Value::Hybrid(crate::primitive::Hybrid { axis: Some(axis), ..h }))
                    } else { Term::Value(Value::Function(self::Function::new(FunctionNode::Axis(left.function()?, axis.clone()), &right_span)?)) }
                }
            }
            (Array, Array) => {
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![Element::Nested(left.array()?)] };
                match right.term { Term::Strand(rest) => items.extend(rest), _ => items.push(Element::Nested(right.array()?)) }
                Term::Strand(items)
            }
            (Function | Hybrid, Hybrid) => {
                let Value::Hybrid(h) = right.value()? else { unreachable!() };
                Term::Value(Value::Function(self::Function::new(FunctionNode::Fold(left.function()?, h), &right_span)?))
            }
            (Array | Function | Hybrid, Operator) => {
                let operand = Operand::from_value(left.value()?);
                let Value::Operator(operator) = right.value()? else { unreachable!() };
                Term::Value(Value::Function(operator.derive(operand, &span)?))
            }
            (DyadicOperator, Array | Function | Hybrid) => {
                let Value::Operator(operator) = left.value()? else { unreachable!() };
                Term::Value(Value::Operator(self::Operator::Bound(Box::new(operator), Operand::from_value(right.value()?))))
            }
            (DyadicOperator, Operator) => {
                let Value::Operator(self::Operator::Primitive(OperatorKind::Compose)) = left.value()? else {
                    return Err(span.error(ErrorKind::Syntax, "only jot can be an operator operand"));
                };
                let Value::Operator(self::Operator::Bound(op, operand)) = right.value()? else {
                    return Err(span.error(ErrorKind::Syntax, "jot needs a product operator"));
                };
                if !matches!(*op, self::Operator::Primitive(OperatorKind::Product)) {
                    return Err(span.error(ErrorKind::Syntax, "jot needs a product operator"));
                }
                let operand = operand.normalize(&span)?;
                if !matches!(operand, Operand::Function(_)) { return Err(span.error(ErrorKind::Domain, "outer product needs a function")); }
                Term::Value(Value::Function(self::Function::new(FunctionNode::Modified(OperatorKind::Outer, operand), &span)?))
            }
            (Array, Function | Hybrid) => Term::Left(left.array()?, right.function()?),
            (Function | Left, Array) => {
                let (x, f) = if let Term::Left(x, f) = left.term { (Some(x), f) } else { (None, left.function()?) };
                let y = right.array()?;
                return Ok(Some(Application { function: f, left: x, right: y, span, unshy: false, selected }));
            }
            (Function | Hybrid, Function) => {
                let mut fs = match left.term { Term::Train(fs) => fs, _ => vec![left.function()?] };
                fs.push(right.function()?);
                Term::Train(fs)
            }
            (Left, Function) => {
                let Term::Left(a, f) = left.term else { unreachable!() };
                let constant = self::Function::new(FunctionNode::Modified(OperatorKind::Commute, Operand::Array(a)), &span)?;
                Term::Train(vec![constant, f, right.function()?])
            }
            _ => return Err(span.error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        // Function/operator location, rather than an attached left argument, owns a call.
        let span = if matches!(term, Term::Left(..)) { right_span } else { span };
        self.stack.push(Entity { term, span, shy: false, selected, assignment: false });
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_construction_shares_both_arms() {
        let mut s = Session::new();
        assert!(s.eval("f←+").error.is_none());
        for _ in 0..64 {
            let Value::Function(previous) = s.names["f"].clone() else { unreachable!() };
            assert!(s.eval("f←f+f").error.is_none());
            let Value::Function(f) = &s.names["f"] else { unreachable!() };
            let FunctionNode::Fork(arms) = f.node.as_ref() else { unreachable!() };
            assert!(Rc::ptr_eq(&arms[0].node, &previous.node));
            assert!(Rc::ptr_eq(&arms[2].node, &previous.node));
        }
    }
    #[test]
    fn active_lexical_frames_are_reclaimed_during_one_input() {
        let mut s = Session::new();
        let body = format!("outer←{{x←2 ⋄ add←{{x+⍵}} ⋄ {} add 3}}", "r←add 3 ⋄ ".repeat(1_000));
        assert!(s.eval(&body).error.is_none());
        let source = Source::new("calls", "outer 0 ⋄ outer 0");
        let weak = Rc::downgrade(&source);
        let result = s.eval_source(source);
        assert!(result.error.is_none());
        assert_eq!(result.value.unwrap(), Array::scalar(5.0).unwrap());
        assert_eq!(s.peak_frames, 2);
        assert!(s.frames.is_empty());
        assert!(s.current.is_none());
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn tail_calls_reclaim_frames_but_keep_lexical_dependencies() {
        for (code, expected, frames) in [
            ("count←{⍺←0 ⋄ ⍵=0:⍺ ⋄ (⍺+1)∇⍵-1} ⋄ count 10000", 10000., 1),
            ("even←{⍵=0:1 ⋄ odd ⍵-1} ⋄ odd←{⍵=0:0 ⋄ even ⍵-1} ⋄ even 10000", 1., 1),
            ("outer←{x←42 ⋄ loop←{⍵=0:x ⋄ ∇⍵-1} ⋄ loop ⍵} ⋄ outer 10000", 42., 2),
            ("outer←{x←42 ⋄ op←{⍵=0:⍺⍺ ⍵ ⋄ ∇⍵-1} ⋄ ({x}op)⍵} ⋄ outer 10000", 42., 2),
            ("loop←{⍵=0:a←7 ⋄ (∇⍵-1)} ⋄ loop 10000", 7., 1),
        ] {
            let mut s = Session::new();
            let result = s.eval(code);
            assert!(result.error.is_none(), "{code}: {:?}", result.error);
            assert_eq!(result.value.unwrap(), Array::scalar(expected).unwrap());
            assert_eq!(s.peak_frames, frames);
            assert!(s.frames.is_empty() && s.current.is_none());
        }
        let mut s = Session::new();
        assert_eq!(s.eval("f←{11::7 ⋄ ⍵=0:1÷0 ⋄ ∇⍵-1} ⋄ f 5").value.unwrap(), Array::scalar(7.).unwrap());
        assert_eq!(s.eval("f 500").error.unwrap().kind, ErrorKind::Limit);
        assert!(s.frames.is_empty() && s.current.is_none());
    }
}
