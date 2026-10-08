use super::*;

pub(super) enum Expression {
    Atom(Operand),
    Operator(String),
    Call { function: Function, left: Option<Box<Self>>, right: Box<Self> },
    Strand(Vec<Self>),
}

impl Expression {
    pub(super) fn binding(binding: &Binding) -> Option<Self> {
        match binding {
            Binding::Value(a) => Some(Self::array(a)),
            Binding::Function(f) => Some(Self::Atom(Operand::Function(f.clone()))),
            Binding::Operator(op) => Some(Self::Operator(op.text())),
            _ => None,
        }
    }
    pub(super) fn array(value: &Value) -> Self { Self::Atom(Operand::Value(value.clone())) }
    pub(super) fn call(function: &Function, left: Option<Self>, right: Self) -> Self {
        Self::Call { function: function.clone(), left: left.map(Box::new), right: Box::new(right) }
    }
    pub(super) fn items(expression: Option<Self>, items: &[Value]) -> Vec<Self> {
        match expression { Some(Self::Strand(items)) => items, Some(item) => vec![item], None => items.iter().map(Self::array).collect() }
    }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        use crate::display::Tree;
        if *budget == 0 { return Tree::leaf("…"); }
        *budget -= 1;
        match self {
            Self::Atom(a) => a.tree(budget),
            Self::Operator(op) => Tree::leaf(op),
            Self::Call { function, left, right } => {
                let mut children = vec![function.tree(budget)];
                children.extend(left.iter().map(|a| a.tree(budget)));
                children.push(right.tree(budget));
                Tree { label: "call".into(), children }
            }
            Self::Strand(items) => Tree { label: "[]".into(), children: items.iter().map(|a| a.tree(budget)).collect() },
        }
    }
    fn text(&self, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        match self {
            Self::Atom(a) => a.text(budget),
            Self::Operator(op) => op.clone(),
            Self::Call { function, left, right } => {
                let left = left.as_ref().map_or(String::new(), |a| a.text(budget));
                format!("({left}({}){})", function.text(budget), right.text(budget))
            }
            Self::Strand(items) => format!("[{}]", items.iter().map(|a| a.text(budget)).collect::<Vec<_>>().join(" ")),
        }
    }
    pub(super) fn display(&self, trees: bool) -> String {
        if trees { return self.tree(&mut 1000).render(); }
        let text = self.text(&mut 1000);
        if matches!(self, Self::Call { .. }) { text[1..text.len() - 1].into() } else { text }
    }
}
