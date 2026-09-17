use crate::{
    primitive::Primitive,
    syntax::{Definition, DefinitionKind, Node, NodeKind},
    Array, Element, Error, ErrorKind, ParseStatus, Parsed, Source, Span,
};
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, Debug)]
struct Function { node: Rc<FunctionNode>, depth: usize }

const MAX_DEPTH: usize = 128;

#[derive(Debug)]
enum FunctionNode {
    Primitive(Primitive),
    Reduce(Function),
    Defined(Closure),
    Derived(Closure, Operand),
    Fork([Function; 3]),
}

impl Function {
    fn primitive(p: Primitive) -> Self { Self { node: Rc::new(FunctionNode::Primitive(p)), depth: 1 } }
    fn new(node: FunctionNode, span: &Span) -> Result<Self, Error> {
        let depth = 1 + match &node {
            FunctionNode::Reduce(f) | FunctionNode::Derived(_, Operand::Function(f)) => f.depth,
            FunctionNode::Fork(fs) => fs.iter().map(|f| f.depth).max().unwrap(),
            _ => 0,
        };
        if depth > MAX_DEPTH { return Err(span.error(ErrorKind::Limit, "function structure exceeds 128 levels")); }
        Ok(Self { node: Rc::new(node), depth })
    }
    fn call(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Array, Error> {
        if session.depth == MAX_DEPTH { return Err(span.error(ErrorKind::Limit, "evaluation depth exceeds 128")); }
        session.depth += 1;
        let result = self.apply(left, right, span, session, output);
        session.depth -= 1;
        result
    }
    fn apply(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Array, Error> {
        use FunctionNode::{Defined, Derived, Fork, Reduce};
        match self.node.as_ref() {
            FunctionNode::Primitive(p) => p.call(left, right, span),
            Defined(_) | Derived(..) => session.call_defined(self, left, right, output).map_err(|mut e| {
                e.calls.push(span.clone());
                e
            }),
            Fork(fns) => {
                let y = fns[2].call(left, right, span, session, output)?;
                let x = fns[0].call(left, right, span, session, output)?;
                fns[1].call(Some(&x), &y, span, session, output)
            }
            Reduce(operand) => {
                if left.is_some() { return Err(span.error(ErrorKind::Unsupported, "n-wise reduction is not implemented yet")); }
                if right.shape().len() > 1 { return Err(span.error(ErrorKind::Unsupported, "reduction along matrix axes is not implemented yet")); }
                let mut items = right.data().iter().rev();
                let Some(last) = items.next() else {
                    let identity = match operand.node.as_ref() {
                        FunctionNode::Primitive(Primitive::Arithmetic(crate::number::Arithmetic::Plus)) => 0,
                        FunctionNode::Primitive(Primitive::Arithmetic(crate::number::Arithmetic::Times)) => 1,
                        _ => return Err(span.error(ErrorKind::Unsupported, "empty reduction currently supports only + and ×")),
                    };
                    let Element::Number(n) = right.prototype() else { return Err(span.error(ErrorKind::Domain, "expected numeric prototype")); };
                    return Ok(Array::scalar(n.unit(identity)).unwrap());
                };
                let scalar = |item: &Element| Array::new(vec![], vec![item.clone()]).map_err(|k| span.error(k, "invalid reduction item"));
                let mut result = scalar(last)?;
                for item in items { result = operand.call(Some(&scalar(item)?), &result, span, session, output)?; }
                Ok(result)
            }
        }
    }
}

// A lexical link is an index into active frames, never an owning reference.
// Functions cannot escape: results/array elements are arrays, assignments are local,
// and the public API exports no functions. Revisit this proof before adding an outlet.
#[derive(Clone, Debug)]
struct Closure { definition: Rc<Definition>, environment: Option<usize> }
struct Frame { names: HashMap<String, Value>, parent: Option<usize> }

#[derive(Clone, Debug)]
enum Operand { Array(Array), Function(Function) }

impl Operand { fn value(&self) -> Value { match self { Self::Array(a) => Value::Array(a.clone()), Self::Function(f) => Value::Function(f.clone()) } } }

#[derive(Clone, Debug)]
enum Value {
    Array(Array),
    Function(Function),
    Operator(Closure),
    Hybrid,
}
struct Bound { value: Value, shy: bool }

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation { pub value: Option<Array>, pub output: Vec<String>, pub error: Option<Error> }

#[derive(Default)]
pub struct Session {
    names: HashMap<String, Value>,
    frames: Vec<Frame>,
    current: Option<usize>,
    depth: usize,
    #[cfg(test)]
    peak_frames: usize,
}

impl Session {
    pub fn new() -> Self { Self::default() }
    pub fn eval(&mut self, code: &str) -> Evaluation { self.eval_source(Source::new("<input>", code)) }
    pub fn eval_source(&mut self, source: std::rc::Rc<Source>) -> Evaluation {
        match crate::parse(source) {
            ParseStatus::Complete(parsed) => self.eval_parsed(&parsed),
            ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => Evaluation { error: Some(e), ..Evaluation::default() },
        }
    }
    pub fn eval_parsed(&mut self, parsed: &Parsed) -> Evaluation {
        let mut result = Evaluation::default();
        for statement in &parsed.statements {
            let nodes = &statement.nodes;
            match self.bind(nodes, &mut result.output) {
                Ok(bound) => {
                    result.value = match bound.value {
                        Value::Array(a) => {
                            if !bound.shy { result.output.push(a.to_string()); }
                            Some(a)
                        }
                        _ if bound.shy => None,
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
    fn bind(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Bound, Error> {
        if self.depth == MAX_DEPTH { return Err(nodes[0].span.error(ErrorKind::Limit, "evaluation depth exceeds 128")); }
        self.depth += 1;
        let result = self.bind_expression(nodes, output);
        self.depth -= 1;
        result
    }

    fn bind_expression(&mut self, mut nodes: &[Node], output: &mut Vec<String>) -> Result<Bound, Error> {
        // Peel assignment targets without recursive bind calls. Install from right to left.
        let mut targets = Vec::new();
        while nodes.len() >= 2 && matches!(nodes[1].kind, NodeKind::Assign) {
            if nodes.len() == 2 { return Err(nodes[1].span.error(ErrorKind::Syntax, "assignment needs a value")); }
            targets.push(&nodes[0]);
            nodes = &nodes[2..];
        }
        let value = Binder::evaluate(nodes, self, output)?;
        for target in targets.iter().rev() {
            match &target.kind {
                NodeKind::Name(name) => {
                    if matches!(name.as_str(), "⍺" | "⍵" | "⍺⍺" | "⍵⍵" | "∇" | "∇∇") {
                        return Err(target.span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned; default ⍺ is not implemented yet"));
                    }
                    let names = match self.current { Some(i) => &mut self.frames[i].names, None => &mut self.names };
                    names.insert(name.clone(), value.clone());
                }
                NodeKind::Output => match &value {
                    Value::Array(a) => output.push(a.to_string()),
                    _ => return Err(target.span.error(ErrorKind::Domain, "output requires an array")),
                },
                _ => return Err(target.span.error(ErrorKind::Syntax, "assignment needs a name or ⎕")),
            }
        }
        Ok(Bound { value, shy: !targets.is_empty() })
    }

    // Resolving one structural item may execute a group, but never derives an operator
    // or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    fn resolve(&mut self, node: &Node, output: &mut Vec<String>) -> Result<Value, Error> {
        Ok(match &node.kind {
            NodeKind::Number(n) => Value::Array(Array::scalar(n.clone()).map_err(|k| node.span.error(k, "invalid number"))?),
            NodeKind::Function(p) => Value::Function(Function::primitive(*p)),
            NodeKind::Hybrid => Value::Hybrid,
            NodeKind::Name(name) => self.lookup(name).cloned().ok_or_else(|| node.span.error(ErrorKind::Value, format!("undefined name: {name}")))?,
            NodeKind::Group(nodes) => self.bind(nodes, output)?.value,
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.current };
                match definition.kind {
                    DefinitionKind::Function => Value::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator => Value::Operator(closure),
                    DefinitionKind::DyadicOperator => {
                        return Err(definition.span.error(ErrorKind::Unsupported, "dyadic defined operators are not implemented yet"))
                    }
                }
            }
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment or output symbol")),
        })
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].names.get(name) { return Some(value); }
            scope = self.frames[i].parent;
        }
        self.names.get(name)
    }

    fn call_defined(&mut self, function: &Function, left: Option<&Array>, right: &Array, output: &mut Vec<String>) -> Result<Array, Error> {
        let (closure, operand) = match function.node.as_ref() {
            FunctionNode::Defined(c) => (c, None),
            FunctionNode::Derived(c, operand) => (c, Some(operand)),
            _ => unreachable!(),
        };
        let mut names = HashMap::from([("⍵".into(), Value::Array(right.clone())), ("∇".into(), Value::Function(function.clone()))]);
        if let Some(a) = left { names.insert("⍺".into(), Value::Array(a.clone())); }
        if let Some(f) = operand { names.insert("⍺⍺".into(), f.value()); }
        let caller = self.current.replace(self.frames.len());
        self.frames.push(Frame { names, parent: closure.environment });
        #[cfg(test)]
        { self.peak_frames = self.peak_frames.max(self.frames.len()); }
        let result = self.run_definition(&closure.definition, output);
        self.frames.pop();
        self.current = caller;
        result
    }

    fn array_result(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Array, Error> {
        match self.bind(nodes, output)?.value { Value::Array(a) => Ok(a), _ => Err(nodes[0].span.error(ErrorKind::Syntax, "dfn results must be arrays")) }
    }

    fn run_definition(&mut self, definition: &Definition, output: &mut Vec<String>) -> Result<Array, Error> {
        // Guards are dynamic call state. A small binding-map checkpoint is sufficient
        // for this slice: values share storage, and restoration also removes new names.
        // This snapshots rollback state, NOT lexical captures (which use live frames).
        let frame = self.current.unwrap();
        let mut handlers = Vec::new();
        let result = (|| {
            for statement in &definition.body.statements {
                let nodes = &statement.nodes;
                if let Some((i, error_guard)) = statement.guard {
                    let condition = self.array_result(&nodes[..i], output)?;
                    let condition = condition
                        .as_number()
                        .ok_or_else(|| nodes[i].span.error(ErrorKind::Domain, "guard requires a Boolean scalar; only 0:: error guards are implemented"))?;
                    let condition = condition.nonnegative_integer().map_err(|k| nodes[i].span.error(k, "invalid guard condition"))?;
                    if error_guard {
                        if condition != 0 { return Err(nodes[i].span.error(ErrorKind::Unsupported, "only catch-all 0:: error guards are implemented yet")); }
                        handlers.push((&nodes[i + 1..], self.frames[frame].names.clone()));
                    } else {
                        if condition > 1 { return Err(nodes[i].span.error(ErrorKind::Domain, "guard requires 0 or 1")); }
                        if condition == 1 { return self.array_result(&nodes[i + 1..], output); }
                    }
                } else {
                    let bound = self.bind(nodes, output)?;
                    if !bound.shy {
                        return match bound.value { Value::Array(a) => Ok(a), _ => Err(nodes[0].span.error(ErrorKind::Syntax, "dfn results must be arrays")) };
                    }
                }
            }
            Err(definition.span.error(ErrorKind::Unsupported, "definitions currently require an explicit array result"))
        })();
        let mut result = result;
        while result.is_err() {
            // Unsupported subset features must not turn into plausible successful results.
            if result.as_ref().unwrap_err().kind == ErrorKind::Unsupported { break; }
            let Some((handler, checkpoint)) = handlers.pop() else { break; };
            self.frames[frame].names = checkpoint;
            result = self.array_result(handler, output);
        }
        result
    }
}

// Only the binder has unfinished strands, trains, and bound left arguments.
// Names and groups contain completed Values, so grouping never flattens a strand.
#[derive(Clone, Copy)]
enum Category {
    Array,
    Function,
    Hybrid,
    Operator,
    Left,
}
enum Term {
    Value(Value),
    Strand(Vec<Element>),
    Train(Vec<Function>),
    Left(Array, Function),
}
struct Entity { term: Term, span: Span }

impl Entity {
    fn category(&self) -> Category {
        match self.term {
            Term::Value(Value::Array(_)) | Term::Strand(_) => Category::Array,
            Term::Value(Value::Function(_)) | Term::Train(_) => Category::Function,
            Term::Value(Value::Hybrid) => Category::Hybrid,
            Term::Value(Value::Operator(_)) => Category::Operator,
            Term::Left(..) => Category::Left,
        }
    }
    fn value(self) -> Result<Value, Error> {
        Ok(match self.term {
            Term::Value(v) => v,
            Term::Strand(items) => Value::Array(Array::new(vec![items.len()], items).map_err(|k| self.span.error(k, "invalid strand"))?),
            Term::Train(fs) => {
                let fs = fs.try_into().map_err(|_| self.span.error(ErrorKind::Unsupported, "only three-function trains are implemented yet"))?;
                Value::Function(self::Function::new(FunctionNode::Fork(fs), &self.span)?)
            }
            Term::Left(..) => return Err(self.span.error(ErrorKind::Syntax, "a function needs a right argument")),
        })
    }
    fn function(self) -> Result<Function, Error> {
        match self.value()? { Value::Function(f) => Ok(f), Value::Hybrid => Ok(Function::primitive(Primitive::Replicate)), _ => unreachable!() }
    }
    fn array(self) -> Result<Array, Error> { match self.value()? { Value::Array(a) => Ok(a), _ => unreachable!() } }
}

// Implemented rows of Dyalog 20's category table, not per-glyph precedence.
fn strength(left: &Entity, right: &Entity) -> u8 {
    use Category::*;
    match (left.category(), right.category()) {
        (Array, Array) => 6,
        (Array | Function | Hybrid, Operator) | (Function | Hybrid | Operator, Hybrid) => 4,
        (Array, Function | Hybrid) => 3,
        (Function | Left, Array) => 2,
        (Function | Hybrid | Left, Function) => 1,
        _ => 0,
    }
}

struct Binder { stack: Vec<Entity> }
impl Binder {
    fn evaluate(nodes: &[Node], session: &mut Session, output: &mut Vec<String>) -> Result<Value, Error> {
        let mut binder = Self { stack: Vec::new() };
        for node in nodes.iter().rev() {
            let left = Entity { term: Term::Value(session.resolve(node, output)?), span: node.span.clone() };
            while binder.stack.len() >= 2 {
                let n = binder.stack.len();
                // Bind the rightmost peak; equal-strength bonds accumulate to the left.
                if strength(&left, &binder.stack[n - 1]) >= strength(&binder.stack[n - 1], &binder.stack[n - 2]) { break; }
                binder.reduce(session, output)?;
            }
            binder.stack.push(left);
        }
        while binder.stack.len() > 1 { binder.reduce(session, output)?; }
        binder.stack.pop().expect("nonempty expression").value()
    }

    fn reduce(&mut self, session: &mut Session, output: &mut Vec<String>) -> Result<(), Error> {
        use Category::*;
        let left = self.stack.pop().unwrap();
        let right = self.stack.pop().unwrap();
        let span = left.span.clone();
        let right_span = right.span.clone();
        let term = match (left.category(), right.category()) {
            (Array, Array) => {
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![Element::Nested(left.array()?)] };
                items.push(Element::Nested(right.array()?));
                Term::Strand(items)
            }
            (Function | Hybrid, Hybrid) => Term::Value(Value::Function(self::Function::new(FunctionNode::Reduce(left.function()?), &right.span)?)),
            (Array | Function | Hybrid, Operator) => {
                let operand = if matches!(left.category(), Array) { Operand::Array(left.array()?) } else { Operand::Function(left.function()?) };
                let Value::Operator(definition) = right.value()? else { unreachable!() };
                Term::Value(Value::Function(self::Function::new(FunctionNode::Derived(definition, operand), &span)?))
            }
            (Array, Function | Hybrid) => Term::Left(left.array()?, right.function()?),
            (Function, Array) => Term::Value(Value::Array(left.function()?.call(None, &right.array()?, &span, session, output)?)),
            (Left, Array) => {
                let Term::Left(x, f) = left.term else { unreachable!() };
                Term::Value(Value::Array(f.call(Some(&x), &right.array()?, &span, session, output)?))
            }
            (Function | Hybrid, Function) => {
                let mut fs = match left.term { Term::Train(fs) => fs, _ => vec![left.function()?] };
                fs.push(right.function()?);
                Term::Train(fs)
            }
            (Left, Function) => return Err(span.error(ErrorKind::Unsupported, "constant train arms are not implemented yet")),
            _ => return Err(span.error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        // Function/operator location, rather than an attached left argument, owns a call.
        let span = if matches!(term, Term::Left(..)) { right_span } else { span };
        self.stack.push(Entity { term, span });
        Ok(())
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
}
