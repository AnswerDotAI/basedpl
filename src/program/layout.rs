//! One key policy: structural facts, 0/1/more lengths, and parameters by read purpose.
use crate::error::OrInternal;
use super::{Desc, Origin, Key, Length, Live, Mode};
use crate::program::trace::ReadKind;
use crate::{
    Function, Number, Value, array::{Layout, Renderer, Trees}, error::Result, eval::OperatorKind, graph::Pending, primitive::{Operation, Primitive, Roles}
};
use std::{borrow::Cow, hash::{Hash, Hasher}, sync::Arc};

pub(crate) type KeyValue = fn(&mut Encoder, &Value, ReadKind) -> Result<()>;

fn stored_parameter(value: &Value) -> Result<Cow<'_, Value>> {
    let mut stored = Cow::Borrowed(value);
    while let Value::Pending(pending) = stored.as_ref() {
        let source = pending.source();
        let value = source.node.is_complete().then(|| source.output.and_then(|output| source.node.stored(output))).flatten().or_internal()?;
        stored = Cow::Owned(value);
    }
    if matches!(stored.as_ref(), Value::Carried { .. }) { return Err(crate::Error::internal()); }
    Ok(stored)
}

pub(crate) struct WeakKey<T>(std::sync::Weak<T>);
impl<T> WeakKey<T> { fn alive(&self) -> bool { self.0.strong_count() != 0 } }
impl<T> Clone for WeakKey<T> { fn clone(&self) -> Self { Self(self.0.clone()) } }
impl<T> PartialEq for WeakKey<T> { fn eq(&self, other: &Self) -> bool { self.0.ptr_eq(&other.0) } }
impl<T> Eq for WeakKey<T> {}
impl<T> Hash for WeakKey<T> { fn hash<H: Hasher>(&self, state: &mut H) { self.0.as_ptr().hash(state); } }

#[derive(Clone)]
pub(crate) struct CaptureKey(Arc<crate::eval::CaptureBindings>);
impl CaptureKey {
    fn of(bindings: &crate::eval::CaptureBindings) -> Self {
        let empty = crate::eval::CaptureBindings::empty();
        Self(if bindings.structure_eq(&empty) { empty } else { Arc::new(bindings.stripped()) })
    }
}
impl PartialEq for CaptureKey { fn eq(&self, other: &Self) -> bool { self.0.structure_eq(&other.0) } }
impl Eq for CaptureKey {}
impl Hash for CaptureKey { fn hash<H: Hasher>(&self, state: &mut H) { self.0.structure_hash(state); } }

#[derive(Clone)]
pub(crate) struct Scalar(Number);
impl PartialEq for Scalar { fn eq(&self, other: &Self) -> bool { self.0.identical(&other.0) && self.0.is_weak() == other.0.is_weak() } }
impl Eq for Scalar {}
impl Hash for Scalar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.identity_hash(state);
        self.0.is_weak().hash(state);
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Axis { class: u8, equal: Option<u32>, name: Option<Arc<str>>, keys: Option<Vec<Option<Arc<str>>>> }
impl Hash for Axis {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.class.hash(state);
        self.equal.hash(state);
        self.name.hash(state);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Count { Finite(Length, bool), Infinite(bool) }

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct LayoutKey { axes: Vec<Axis>, renderer: Option<Renderer> }

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Parameter {
    Number(Scalar), Character(char), Array(LayoutKey, Vec<Parameter>), Function(Vec<KeyPart>), Operator(Vec<KeyPart>),
    Counts { layout: LayoutKey, lengths: Vec<Count> }, Kind(crate::number::Kind)
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum KeyPart {
    Stored(bool), Opaque(bool), Publishing(bool), Graph, Call, Node(usize), Output(Option<u32>), Source(usize), Operation(Operation), Mode(Mode), Roles(Roles),
    Arguments(usize, usize), Trees(Option<Arc<Trees>>), Captures(CaptureKey), Description { kind: crate::number::Kind, atom: bool, layout: LayoutKey },
    Shape(Vec<usize>), Physical(Vec<(u8, usize)>), Parameter(ReadKind, Parameter), Missing, SourceBase(bool, Option<Arc<str>>), Count(usize, Arc<[KeyPart]>), Primitive(Primitive),
    Operator(OperatorKind), Definition(WeakKey<crate::syntax::Definition>), Recorded(WeakKey<crate::eval::RecordedCode>), System(crate::system::SystemFunction),
    Train(usize), Array, Operand { structural: bool }, End
}

#[derive(Default)]
pub(crate) struct Encoder { pub(crate) parts: Vec<KeyPart>, pub(super) lengths: Vec<usize> }

impl Encoder {
    pub(crate) fn push(&mut self, part: KeyPart) { self.parts.push(part); }
    pub(crate) fn inputs(values: &[Value]) -> Self {
        let mut encoder = Self::default();
        for value in values { if let Some(desc) = Desc::of(value) { encoder.description(&desc); } }
        encoder.parts.clear();
        encoder
    }
    pub(crate) fn count(&mut self, stored: &Value) -> Result<Key> {
        debug_assert!(self.parts.is_empty());
        self.value(stored, ReadKind::Count)?;
        Ok(Key(std::mem::take(&mut self.parts).into()))
    }
    pub(crate) fn definition(&mut self, definition: &Arc<crate::syntax::Definition>) { self.push(KeyPart::Definition(WeakKey(Arc::downgrade(definition)))); }
    pub(crate) fn recorded(&mut self, code: &Arc<crate::eval::RecordedCode>) { self.push(KeyPart::Recorded(WeakKey(Arc::downgrade(code)))); }
    fn context(&mut self, operation: &Operation, site: Option<&crate::Span>) {
        if matches!(operation, Operation::System | Operation::Effect(_)) {
            let source = site.map(|site| &site.source);
            self.push(KeyPart::SourceBase(
                source.is_some_and(|source| source.file), source.filter(|source| source.file).map(|source| Arc::from(source.name.as_str()))));
        }
    }

    fn layout(&mut self, layout: &Layout) -> LayoutKey {
        let axes = layout.shape().iter().enumerate().map(|(axis, &length)| {
            let key = Axis { class: length.min(2) as u8, equal: self.lengths.iter().position(|&n| n == length).map(|i| i as u32),
                name: layout.name(axis).cloned(), keys: layout.keys(axis).map(|keys| keys.names().to_vec()) };
            self.lengths.push(length);
            key
        }).collect();
        LayoutKey { axes, renderer: layout.renderer_metadata().cloned() }
    }

    pub(crate) fn description(&mut self, desc: &Desc) {
        let layout = self.layout(&desc.layout);
        self.push(KeyPart::Description { kind: desc.kind, atom: desc.atom, layout });
        self.push(KeyPart::End);
    }

    pub(crate) fn value(&mut self, value: &Value, purpose: ReadKind) -> Result<()> {
        self.data(value);
        let stored = value.read_borrowed(purpose)?;
        self.emit(stored.as_ref(), purpose)
    }
    pub(crate) fn stored(&mut self, value: &Value, purpose: ReadKind) -> Result<()> {
        self.data(value);
        self.emit(value, purpose)
    }
    fn emit(&mut self, value: &Value, purpose: ReadKind) -> Result<()> {
        let stored = stored_parameter(value)?;
        let value = stored.as_ref();
        let parameter = match purpose {
            ReadKind::Description | ReadKind::RuntimeOnly => return Err(crate::Error::internal()),
            ReadKind::Kind if value.kind().is_some() => Parameter::Kind(value.kind().unwrap()),
            ReadKind::Count => {
                let layout = self.layout(value.layout());
                let lengths = value.elements().map(|value| {
                    let stored = stored_parameter(&value)?;
                    let number = stored.as_ref().as_number().ok_or_else(|| crate::Error::domain("expected a numeric count"))?;
                    if number.is_infinite() { return Ok(Count::Infinite(number.as_float().unwrap().is_sign_negative())); }
                    let count = number.integer()?;
                    let length = count.unsigned_abs() as usize;
                    let class = Length::of(length, &self.lengths);
                    self.lengths.push(length);
                    Ok(Count::Finite(class, count < 0))
                }).collect::<Result<_>>()?;
                Parameter::Counts { layout, lengths }
            }
            _ => self.parameter(value)?,
        };
        self.push(KeyPart::Parameter(purpose, parameter));
        Ok(())
    }

    fn parameter(&mut self, value: &Value) -> Result<Parameter> {
        let stored = stored_parameter(value)?;
        let value = stored.as_ref();
        Ok(match value {
            Value::Number(number) => Parameter::Number(Scalar(number.clone())),
            Value::Character(character) => Parameter::Character(*character),
            Value::Function(function) => {
                let mut key = Encoder::default();
                function.key_parts(&mut key, Encoder::stored)?;
                Parameter::Function(key.parts)
            }
            Value::Operator(operator) => {
                let mut key = Encoder::default();
                operator.key_parts(&mut key, Encoder::stored)?;
                Parameter::Operator(key.parts)
            }
            Value::Array(_) => Parameter::Array(self.layout(value.layout()), value.elements().map(|item| self.parameter(&item)).collect::<Result<_>>()?),
            Value::Pending(_) | Value::Carried { .. } => return Err(crate::Error::internal()),
        })
    }

    /// Key a function or operator atom by its structure; false for any other value.
    pub(crate) fn callable(&mut self, value: &Value, encode: KeyValue) -> Result<bool> {
        match value {
            Value::Function(function) => function.key_parts(self, encode)?,
            Value::Operator(operator) => operator.key_parts(self, encode)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn data(&mut self, value: &Value) {
        if let Some(desc) = Desc::of(value) { self.description(&desc); }
        else { self.push(KeyPart::Missing); }
        if let Value::Array(_) = value && let Some((_, Some(map))) = value.physical() && let Some((_, strides)) = map.affine(value.shape()) {
            let mut sizes: Vec<_> = strides.iter().map(|step| step.unsigned_abs()).collect();
            sizes.sort_unstable();
            sizes.dedup();
            let order = strides.iter().map(|&step| {
                (if step < 0 { 0 } else if step == 0 { 1 } else { 2 }, sizes.binary_search(&step.unsigned_abs()).unwrap())
            }).collect();
            self.push(KeyPart::Physical(order));
        }
    }

    pub(crate) fn finish(self) -> Key { Key(self.parts.into()) }
}

pub(crate) fn call_key(function: &Function, origin: &Origin<&[Value]>) -> Result<Key> {
    let mut key = Encoder::default();
    key.push(KeyPart::Call);
    function.key_parts(&mut key, Encoder::value)?;
    key.push(KeyPart::Operation(origin.operation.clone()));
    key.push(KeyPart::Mode(origin.mode));
    key.push(KeyPart::Roles(origin.roles.clone()));
    key.push(KeyPart::Arguments(origin.explicit as usize, origin.explicit_roots as usize));
    key.push(KeyPart::Trees(origin.trees.clone()));
    key.push(KeyPart::Captures(CaptureKey::of(&origin.bindings)));
    key.context(&origin.operation, origin.site.as_ref());
    for (position, value) in origin.list.iter().enumerate() {
        if key.callable(value, Encoder::value)? { continue; }
        if value.constant_value().is_some() { key.value(value, ReadKind::Value)?; continue; }
        let read = origin.read_kind(position);
        if read.is_none() || read == Some(ReadKind::Description) { key.data(value); }
        if let Some(read) = read.filter(|read| *read != ReadKind::Description) { key.value(value, read)?; }
    }
    Ok(key.finish())
}

pub(crate) fn guard_key(origin: &super::Origin<&[Value]>, reads: &[super::Parameter]) -> Result<Key> {
    let mut key = Encoder::default();
    for parameter in reads {
        if let super::Parameter::Input { position, read } = parameter {
            let value = origin.list.get(*position).or_internal()?;
            if *read == ReadKind::Description { super::trace::Tracing::shape(origin, value)?; key.push(KeyPart::Shape(value.shape().to_vec())); }
            else { key.value(value, *read)?; }
        }
    }
    Ok(key.finish())
}

pub(crate) fn graph_key(live: &Live) -> Result<Key> {
    let mut key = Encoder::default();
    key.push(KeyPart::Graph);
    let nodes: rustymath::hashing::HashMap<_, _> = live.nodes.iter().cloned().enumerate().map(|(i, node)| (node, i)).collect();
    let reference = |key: &mut Encoder, value: &Value| {
        if let Some(pending) = value.pending() && let Some(&node) = nodes.get(&pending.node) {
            key.push(KeyPart::Node(node));
            key.push(KeyPart::Output(pending.output));
        } else {
            key.push(KeyPart::Source(live.values[&super::Identity::of(value)]));
            key.data(value);
        }
    };
    for node in &live.nodes {
        key.push(KeyPart::Stored(node.completed().is_some()));
        node.with_address_state(|state| -> Result<()> {
            match state {
                crate::graph::State::Step { origin, .. } => {
                    key.push(KeyPart::Publishing(live.publishes(node, origin)));
                    key.push(KeyPart::Operation(origin.operation.clone()));
                    key.push(KeyPart::Mode(origin.mode));
                    key.push(KeyPart::Arguments(origin.explicit as usize, origin.explicit_roots as usize));
                    key.push(KeyPart::Roles(origin.roles.clone()));
                    key.push(KeyPart::Trees(origin.trees.clone()));
                    key.push(KeyPart::Captures(CaptureKey::of(&origin.bindings)));
                    key.context(&origin.operation, origin.site.as_ref());
                    for (i, read) in origin.list.iter().enumerate() {
                        key.push(KeyPart::Opaque(read.opaque));
                        match &read.operand {
                            value @ Value::Pending(_) => {
                                reference(&mut key, value);
                                if let Some(read) = origin.read_kind(i).filter(|read| matches!(read, ReadKind::Structure | ReadKind::Value)) {
                                    key.stored(value, read)?;
                                }
                            }
                            value => {
                                key.push(KeyPart::Source(live.values[&super::Identity::of(value)]));
                                if !key.callable(value, Encoder::stored)? {
                                    if let Some(read) = origin.read_kind(i).filter(|read| *read != ReadKind::Description) {
                                        key.stored(value, read)?;
                                    } else { key.data(value); }
                                }
                            }
                        }
                    }
                }
                crate::graph::State::Input { value, role, .. } => {
                    let pending = if node.descs().is_empty() { Pending::control(node.clone()) } else { Pending::data(node.clone(), 0) };
                    if role == super::trace::Role::Seed || !matches!(value, Some(Value::Pending(_))) {
                        key.push(KeyPart::Source(live.input_position(&pending, value)));
                    }
                    match value {
                        Some(value @ Value::Pending(_)) => reference(&mut key, value), Some(value) => key.data(value),
                        None => match node.desc(0) { Some(desc) => key.description(desc), None => key.push(KeyPart::Missing) }
                    }
                }
                crate::graph::State::Done(values) => {
                    for (i, value) in values.as_ref().iter().enumerate() {
                        if node.is_literal() { key.stored(value, ReadKind::Value)?; } else {
                            key.push(KeyPart::Source(live.input_position(&Pending::data(node.clone(), i as u32), Some(value))));
                            key.data(value);
                        }
                    }
                    if !node.is_literal() { key.push(KeyPart::Source(live.input_position(&Pending::control(node.clone()), None))); }
                    key.push(KeyPart::Output(None));
                }
            }
            Ok(())
        })?;
        key.push(KeyPart::End);
    }
    for root in &live.roots { reference(&mut key, root); }
    Ok(key.finish())
}

pub(crate) fn lifetime(key: &Key) -> Option<bool> {
    fn parameter(value: &Parameter) -> Option<bool> {
        match value {
            Parameter::Function(parts) | Parameter::Operator(parts) => parts_alive(parts),
            Parameter::Array(_, values) => join(values.iter().filter_map(parameter)), _ => None
        }
    }
    fn join(values: impl Iterator<Item = bool>) -> Option<bool> { values.fold(None, |old, live| Some(old.unwrap_or(true) && live)) }
    fn parts_alive(parts: &[KeyPart]) -> Option<bool> {
        join(parts.iter().filter_map(|part| match part {
            KeyPart::Definition(definition) => Some(definition.alive()), KeyPart::Recorded(recorded) => Some(recorded.alive()),
            KeyPart::Parameter(_, value) => parameter(value), _ => None
        }))
    }
    parts_alive(&key.0)
}
