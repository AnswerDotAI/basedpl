//! One key policy: structural facts, 0/1/more lengths, and parameters by read purpose.
use super::{Desc, Key, Length, Live, Mode, ProgramKey};
use crate::{Function, Number, Value, array::{Layout, Renderer, Trees}, error::Result, eval::OperatorKind, primitive::{Operation, Primitive, Roles}};
use crate::program::trace::ReadKind;
use rustymath::{buffer::DType, layout::Keys};
use std::{hash::{Hash, Hasher}, sync::Arc};

#[derive(Clone)]
pub(crate) struct DefinitionKey(std::sync::Weak<crate::syntax::Definition>);
impl PartialEq for DefinitionKey { fn eq(&self, other: &Self) -> bool { self.0.ptr_eq(&other.0) } }
impl Eq for DefinitionKey {}
impl Hash for DefinitionKey { fn hash<H: Hasher>(&self, state: &mut H) { self.0.as_ptr().hash(state); } }

#[derive(Clone)]
pub(crate) struct CaptureKey(Arc<crate::eval::CaptureBindings>);
impl CaptureKey {
    fn of(bindings: &crate::eval::CaptureBindings) -> Self {
        let empty = crate::eval::CaptureBindings::empty(); Self(if bindings.structure_eq(&empty) { empty } else { Arc::new(bindings.stripped()) })
    }
}
impl PartialEq for CaptureKey { fn eq(&self, other: &Self) -> bool { self.0.structure_eq(&other.0) } }
impl Eq for CaptureKey {}
impl Hash for CaptureKey { fn hash<H: Hasher>(&self, state: &mut H) { self.0.structure_hash(state); } }

#[derive(Clone)]
pub(crate) struct Scalar(Number);
impl PartialEq for Scalar { fn eq(&self, other: &Self) -> bool { self.0.identical(&other.0) && self.0.is_weak() == other.0.is_weak() } }
impl Eq for Scalar {}
impl Hash for Scalar { fn hash<H: Hasher>(&self, state: &mut H) { self.0.identity_hash(state); self.0.is_weak().hash(state); } }

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Axis { class: u8, equal: Option<u32>, name: Option<AxisName>, keys: Option<Vec<Option<AxisName>>> }
impl Hash for Axis {
    fn hash<H: Hasher>(&self, state: &mut H) { self.class.hash(state); self.equal.hash(state); self.name.hash(state); }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum AxisName { Given(Arc<str>), Drawn(u32) }
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Count { Finite(Length, bool), Infinite(bool) }


#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct LayoutKey { axes: Vec<Axis>, renderer: Option<Renderer> }

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Parameter {
    Number(Scalar), Character(char), Array(LayoutKey, Vec<Parameter>), Function(Vec<KeyPart>), Operator(Vec<KeyPart>),
    Counts { layout: LayoutKey, lengths: Vec<Count> },
    Kind(crate::number::Kind),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum KeyPart {
    Graph, Call, Node(usize), Output(Option<u32>), Source(usize), Operation(Operation), Mode(Mode), Roles(Roles), Literal(u64),
    Arguments(usize, usize), Trees(Option<Arc<Trees>>), Captures(CaptureKey), Description { kind: crate::number::Kind, atom: bool, layout: LayoutKey },
    Physical(Vec<(u8, usize)>), Parameter(ReadKind, Parameter), Missing,
    SourceBase(bool, Option<Arc<str>>),
    Count(usize, Vec<KeyPart>), Primitive(Primitive), Operator(OperatorKind), Definition(DefinitionKey), System(Arc<str>),
    Train(usize), Array, Operand { structural: bool }, End,
}

#[derive(Default)]
pub(crate) struct Encoder { pub(crate) parts: Vec<KeyPart>, lengths: Vec<usize>, names: Vec<Arc<str>> }

impl Encoder {
    pub(crate) fn push(&mut self, part: KeyPart) { self.parts.push(part); }
    pub(crate) fn inputs(values: &[Value]) -> Self {
        let mut encoder = Self::default();
        for value in values { if let Some(desc) = Desc::of(value) { encoder.description(&desc); } }
        encoder.parts.clear(); encoder
    }
    pub(crate) fn count(&mut self, stored: &Value) -> Result<Key> {
        debug_assert!(self.parts.is_empty());
        self.value(stored, ReadKind::Count)?;
        Ok(Key(std::mem::take(&mut self.parts)))
    }
    pub(crate) fn definition(&mut self, definition: &Arc<crate::syntax::Definition>) {
        self.push(KeyPart::Definition(DefinitionKey(Arc::downgrade(definition))));
    }
    fn context(&mut self, operation: &Operation, site: Option<&crate::Span>) {
        if matches!(operation, Operation::System | Operation::Effect(_)) {
            let source = site.map(|site| &site.source);
            self.push(KeyPart::SourceBase(source.is_some_and(|source| source.file), source.filter(|source| source.file).map(|source| Arc::from(source.name.as_str()))));
        }
    }

    fn name(&mut self, name: &Arc<str>) -> AxisName {
        if !rustymath::layout::hidden(name) || !name.contains(rustymath::layout::FRESH) { return AxisName::Given(name.clone()); }
        let at = self.names.iter().position(|prior| prior == name).unwrap_or_else(|| { self.names.push(name.clone()); self.names.len() - 1 });
        AxisName::Drawn(at as u32)
    }

    fn layout(&mut self, layout: &Layout) -> LayoutKey {
        let axes = layout.shape().iter().enumerate().map(|(axis, &length)| {
            let key = Axis { class: length.min(2) as u8,
                equal: self.lengths.iter().position(|&n| n == length).map(|i| i as u32),
                name: layout.name(axis).map(|name| self.name(name)), keys: layout.keys(axis).map(|keys|
                    keys.names().iter().map(|name| name.as_ref().map(|name| self.name(name))).collect()) };
            self.lengths.push(length); key
        }).collect();
        LayoutKey { axes, renderer: layout.renderer_metadata().cloned() }
    }

    pub(crate) fn description(&mut self, desc: &Desc) {
        let layout = self.layout(&desc.layout);
        self.push(KeyPart::Description { kind: desc.kind, atom: desc.atom, layout });
        self.push(KeyPart::End);
    }

    pub(crate) fn value(&mut self, value: &Value, purpose: ReadKind) -> Result<()> {
        let parameter = match purpose {
            ReadKind::Description | ReadKind::RuntimeOnly => return Err(crate::Error::planning()),
            ReadKind::Kind if value.kind().is_some() => Parameter::Kind(value.kind().unwrap()),
            ReadKind::Count => {
                let layout = self.layout(value.layout());
                let lengths = value.elements().map(|value| {
                    let number = value.as_number().ok_or_else(|| crate::Error::domain("expected a numeric count"))?;
                    if number.is_infinite() { return Ok(Count::Infinite(number.as_float().unwrap().is_sign_negative())); }
                    let count = number.integer()?;
                    let length = count.unsigned_abs() as usize; let class = Length::of(length, &self.lengths);
                    self.lengths.push(length); Ok(Count::Finite(class, count < 0))
                }).collect::<Result<_>>()?;
                Parameter::Counts { layout, lengths }
            }
            _ => self.parameter(value)?,
        };
        self.push(KeyPart::Parameter(purpose, parameter)); Ok(())
    }

    fn parameter(&mut self, value: &Value) -> Result<Parameter> {
        Ok(match value {
            Value::Number(number) => Parameter::Number(Scalar(number.clone())),
            Value::Character(character) => Parameter::Character(*character),
            Value::Function(function) => { let mut key = Encoder::default(); function.key_parts(&mut key)?; Parameter::Function(key.parts) }
            Value::Operator(operator) => { let mut key = Encoder::default(); operator.key_parts(&mut key)?; Parameter::Operator(key.parts) }
            Value::Array(_) => Parameter::Array(self.layout(value.layout()), value.elements().map(|item| self.parameter(&item)).collect::<Result<_>>()?),
            Value::Pending(_) | Value::Carried { .. } => return Err(crate::Error::planning()),
        })
    }

    fn data(&mut self, value: &Value) {
        if let Some(desc) = Desc::of(value) { self.description(&desc); } else { self.push(KeyPart::Missing); }
        if let Value::Array(_) = value && let Some((_, Some(map))) = value.physical()
            && let Some((_, strides)) = map.affine(value.shape()) {
            let mut sizes: Vec<_> = strides.iter().map(|step| step.unsigned_abs()).collect(); sizes.sort_unstable(); sizes.dedup();
            let order = strides.iter().map(|&step| (if step < 0 { 0 } else if step == 0 { 1 } else { 2 },
                sizes.binary_search(&step.unsigned_abs()).unwrap())).collect();
            self.push(KeyPart::Physical(order));
        }
    }

    pub(crate) fn finish(self) -> Key { Key(self.parts) }
}

pub(crate) fn call_key(function: &Function, call: &crate::primitive::CallView<'_>) -> Result<Key> {
    let mut key = Encoder::default();
    key.push(KeyPart::Call); function.key_parts(&mut key)?;
    key.push(KeyPart::Operation(call.operation.clone())); key.push(KeyPart::Mode(call.mode)); key.push(KeyPart::Roles(call.roles.clone()));
    key.push(KeyPart::Arguments(call.explicit, call.explicit_roots)); key.push(KeyPart::Literal(call.literals)); key.push(KeyPart::Trees(call.trees.cloned()));
    key.push(KeyPart::Captures(CaptureKey::of(call.captures)));
    key.context(call.operation, call.site);
    for position in 0..call.len() {
        let value = call.input(position).ok_or_else(crate::Error::planning)?.value;
        let read = super::read_kind(call.operation, call.explicit, call.roles, call.captures, position);
        match value {
            Value::Function(function) => function.key_parts(&mut key)?,
            value if position < 64 && call.literals & (1 << position) != 0 => key.value(value, ReadKind::Value)?,
            value => {
                if read.is_none() || matches!(read, Some(ReadKind::Kind | ReadKind::Description)) { key.data(value); }
                if let Some(read) = read.filter(|read| *read != ReadKind::Description) { key.value(value, read)?; }
            }
        }
    }
    Ok(key.finish())
}

pub(crate) fn guard_key(origin: &super::Origin<&[Value]>, reads: &[super::Parameter]) -> Result<Key> {
    let mut key = Encoder::default();
    for parameter in reads {
        if let super::Parameter::Input { position, read } = parameter {
            let value = origin.list.get(*position).ok_or_else(crate::Error::planning)?.read(*read)?;
            key.value(&value, *read)?;
        }
    }
    Ok(key.finish())
}

pub(crate) fn graph_key(live: &Live) -> ProgramKey {
    let mut key = Encoder::default();
    key.push(KeyPart::Graph);
    let nodes: rustymath::hashing::HashMap<_, _> = live.nodes.iter().cloned().enumerate().map(|(i, node)| (node, i)).collect();
    let reference = |key: &mut Encoder, value: &Value| {
        if let Some(pending) = value.pending() && let Some(&node) = nodes.get(&pending.node) {
            key.push(KeyPart::Node(node)); key.push(KeyPart::Output(pending.output));
        } else {
            key.push(KeyPart::Source(live.values[&super::Identity::of(value)])); key.data(value);
        }
    };
    for node in &live.nodes {
        node.with_state(|state| match state {
            crate::graph::State::Step { origin, .. } => {
                key.push(KeyPart::Operation(origin.operation.clone())); key.push(KeyPart::Mode(origin.mode));
                key.push(KeyPart::Arguments(origin.explicit as usize, origin.explicit_roots as usize)); key.push(KeyPart::Roles(origin.roles.clone())); key.push(KeyPart::Literal(origin.literals)); key.push(KeyPart::Trees(origin.trees.clone()));
                key.push(KeyPart::Captures(CaptureKey::of(&origin.bindings)));
                key.context(&origin.operation, origin.site.as_ref());
                for (i, read) in origin.list.iter().enumerate() {
                    match &read.operand {
                        value @ Value::Pending(_) => reference(&mut key, value),
                        value if i < 64 && origin.literals & (1 << i) != 0 => { key.value(value, ReadKind::Value).expect("a literal is stored"); }
                        value => {
                            key.push(KeyPart::Source(live.values[&super::Identity::of(value)]));
                            if let Value::Function(function) = value { function.key_parts(&mut key).expect("a callable has a structural key"); }
                            else if let Some(read) = origin.read_kind(i).filter(|read| *read != ReadKind::Description) { key.value(value, read).expect("resolved call parameters"); }
                            else { key.data(value); }
                        }
                    }
                }
            }
            crate::graph::State::Input { value, role, .. } => {
                let pending = if node.descs().is_empty() { Pending::control(node.clone()) } else { Pending::data(node.clone(), 0) };
                if *role == super::trace::Role::Seed || !matches!(value, Some(Value::Pending(_))) {
                    key.push(KeyPart::Source(live.input_position(&pending, value)));
                }
                match value {
                    Some(value @ Value::Pending(_)) => reference(&mut key, value),
                    Some(value) => key.data(value), None => match node.desc(0) { Some(desc) => key.description(desc), None => key.push(KeyPart::Missing) },
                }
            }
            crate::graph::State::Done(values) => {
                for (i, value) in values.as_ref().iter().enumerate() {
                    if node.is_literal() { key.value(value, ReadKind::Value).expect("a source literal is stored"); }
                    else { key.push(KeyPart::Source(live.input_position(&Pending::data(node.clone(), i as u32), Some(value)))); key.data(value); }
                }
                if !node.is_literal() { key.push(KeyPart::Source(live.input_position(&Pending::control(node.clone()), None))); }
                key.push(KeyPart::Output(None));
            }
        });
        key.push(KeyPart::End);
    }
    for root in &live.roots { reference(&mut key, root); }
    ProgramKey(key.finish())
}


pub(crate) fn lifetime(key: &Key) -> Option<bool> {
    fn parameter(value: &Parameter) -> Option<bool> {
        match value { Parameter::Function(parts) | Parameter::Operator(parts) => parts_alive(parts),
            Parameter::Array(_, values) => join(values.iter().filter_map(parameter)), _ => None }
    }
    fn join(values: impl Iterator<Item = bool>) -> Option<bool> {
        values.fold(None, |old, live| Some(old.unwrap_or(true) && live))
    }
    fn parts_alive(parts: &[KeyPart]) -> Option<bool> {
        join(parts.iter().filter_map(|part| match part {
            KeyPart::Definition(definition) => Some(definition.0.strong_count() != 0),
            KeyPart::Parameter(_, value) => parameter(value), _ => None,
        }))
    }
    parts_alive(&key.0)
}
