//! Slot frames, call-time capture bindings and stable module entries.
use crate::error::OrInternal;
use super::{Binding, Category, MAX_FRAME_DEPTH};
use crate::{Error, Span, Value, array::{Root, TreeRoot}, graph::Hold, name::Name};
use rustymath::{error::Kind, hashing::{HashMap, HashMapExt, HashSet, HashSetExt}};
use std::{cell::{Cell, RefCell}, ops::Range, sync::atomic::{AtomicU64, Ordering::Relaxed}};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Place { Slot(usize, u32, u64), Other(usize, Name, u64), Entry(usize, u32) }
impl Place {
    pub(crate) fn frame(self) -> Option<usize> { match self { Self::Slot(i, ..) | Self::Other(i, ..) => Some(i), Self::Entry(..) => None } }
}

#[derive(Clone, Debug)]
pub(crate) struct FrameInputs { frame: usize, generation: u64 }
impl FrameInputs {
    pub(super) fn environment(&self) -> Option<usize> { Some(self.frame) }
    pub(super) fn with_origin<R>(&self, run: impl FnOnce(crate::program::Origin<&[Value]>) -> crate::error::Result<R>) -> crate::error::Result<R> {
        super::session().bindings.with_origin(self, run)
    }
    pub(super) fn rebuild(&self, template: &super::Function) -> crate::error::Result<super::Function> {
        self.with_origin(|origin| {
            let first = origin.layout().captures();
            let trees = origin.trees.as_ref().map(|trees| trees.project_roots(&trees.roots().skip(origin.layout().capture_roots().get()).collect::<Vec<_>>()));
            template.rebuilt(&origin.list[first..], trees.as_ref(), &origin.bindings, &crate::program::BindingContext::from_origin(&origin))
        })
    }
}

pub(super) type Callable = super::FunctionBinding<super::Function, FrameInputs>;
impl Callable {
    pub(super) fn function(&self) -> crate::error::Result<super::Function> {
        match self { Self::Input(function) => Ok(function.clone()), Self::Rebuild { template, dependencies } => dependencies.rebuild(template) }
    }
    pub(super) fn definition(&self) -> Option<&crate::syntax::Definition> {
        match self { Self::Input(function) | Self::Rebuild { template: function, .. } => function.definition() }
    }
    pub(super) fn origin(&self, args: &[Value]) -> crate::error::Result<crate::program::Origin<Vec<Value>>> {
        match self {
            Self::Input(function) => function.origin(args),
            Self::Rebuild { template, dependencies } => dependencies.with_origin(|parent| {
                template.call_origin(&parent, args)
            }),
        }
    }
}

const IMPLICIT: usize = 6;
static IDS: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct Unslotted { names: HashMap<Name, (Binding, Option<Hold>)>, record: Option<Value> }

pub(super) struct Frame {
    locals: &'static [Name], base: usize, id: u64, unslotted: Option<Box<Unslotted>>, parent: Option<usize>, module: usize,
    origin: Option<crate::program::Origin<Vec<Value>>>, scope: Option<u32>, roots: Box<[TreeRoot]>
}
impl Frame {
    pub(super) fn new(parent: Option<usize>, module: usize, locals: &'static [Name]) -> Self {
        Self { locals, base: 0, id: IDS.fetch_add(1, Relaxed), unslotted: None, parent, module, origin: None, scope: None, roots: Box::default() }
    }
    pub(super) fn scope(parent: Option<usize>, module: usize, record: Value) -> Self {
        Self { unslotted: Some(Box::new(Unslotted { names: HashMap::new(), record: Some(record) })), ..Self::new(parent, module, &[]) }
    }
    fn slots(&self) -> Range<usize> { self.base..self.captured() + self.roots.len() }
    fn captured(&self) -> usize { self.base + IMPLICIT + self.locals.len() }
    fn inputs(&self, frame: usize) -> FrameInputs { FrameInputs { frame, generation: self.id } }
    fn closed(&self) -> Option<&super::CaptureScope> { self.origin.as_ref()?.bindings.scopes.get(self.scope? as usize) }
    fn root_slot(&self, position: &TreeRoot) -> Option<usize> {
        let origin = self.origin.as_ref()?;
        let index = origin.layout().capture_roots().after(position.index.get()).get();
        self.roots.get(index).map(|_| self.captured() + index)
    }
    fn root_value(&self, slot: usize, slots: &[Option<Binding>]) -> Result<Option<Binding>, Error> {
        if let Some(binding) = &slots[slot] { return Ok(Some(binding.clone())); }
        let Some(origin) = &self.origin else { return Ok(None) };
        let root = &self.roots[slot - self.captured()];
        let value = origin.trees.as_ref().map_or_else(|| origin.list[root.leaves.start].clone(), |trees| trees.at_root(root, &origin.list).into_owned());
        Ok(Some(Binding::Value(value)))
    }
    fn root_binding(&self, frame: usize, slot: usize, slots: &[Option<Binding>]) -> Result<Option<Binding>, Error> {
        Ok(self.root_value(slot, slots)?.map(|binding| match binding {
            Binding::Value(Value::Function(function)) if self.origin.as_ref().is_some_and(|origin| origin.bindings.scope(&function).is_some()) => {
                Binding::Function(super::FunctionBinding::Rebuild { template: function, dependencies: self.inputs(frame) })
            }
            Binding::Value(Value::Function(function)) => function.into(),
            Binding::Value(Value::Operator(operator)) => Binding::Operator((*operator).clone()),
            binding => binding,
        }))
    }
    fn slot(&self, name: Name) -> Option<u32> {
        name.implicit().map(|k| k as u32).or_else(|| self.locals.binary_search(&name).ok().map(|k| (IMPLICIT + k) as u32))
    }
    fn place(&self, i: usize, name: Name) -> Place { self.slot(name).map_or(Place::Other(i, name, self.id), |slot| Place::Slot(i, slot, self.id)) }
    fn other(&self, name: Name) -> Option<&Binding> { self.unslotted.as_ref()?.names.get(&name).map(|(binding, _)| binding) }
    fn key(&self, name: Name) -> Result<Option<Binding>, Error> {
        let Some(record) = self.unslotted.as_ref().and_then(|unslotted| unslotted.record.as_ref()) else { return Ok(None) };
        if record.keys(0).and_then(|keys| keys.position(name.text())).is_none() { return Ok(None); }
        let operation = crate::primitive::Operation::Primitive { primitive: crate::primitive::Primitive::Mix, pairing: crate::primitive::Pairing::Axes };
        Ok(Some(Binding::from_element(operation.call(&[Value::constant(crate::keyed::text(name.text())), record.clone()])?)))
    }
    fn names<'a>(&'a self, slots: &'a [Option<Binding>]) -> impl Iterator<Item = &'a str> {
        let locals = self.locals.iter().zip(&slots[self.base + IMPLICIT..]).filter(|(_, slot)| !matches!(slot, None | Some(Binding::Released)))
            .map(|(name, _)| name.text());
        let captured = self.closed().into_iter().flat_map(|scope| scope.names.iter())
            .filter_map(move |name| self.root_slot(name.position.as_ref()?).and_then(|slot| slots[slot].as_ref()).map(|_| name.name.text()));
        let keys = |record: &'a Value| record.keys(0).into_iter().flat_map(|keys| keys.names().iter().flatten().map(|key| &**key));
        locals.chain(captured).chain(self.unslotted.iter().flat_map(move |u| u.names.keys().map(|name| name.text()).chain(u.record.iter().flat_map(keys))))
    }
}

#[derive(Default)]
struct Stack { frames: Vec<Frame>, slots: Slots }

/// Slots that hold bindings. A pending value in a slot is bound (`Hold`) until the slot is overwritten, cleared or dropped.
#[derive(Default)]
struct Slots { bindings: Vec<Option<Binding>>, holds: Vec<Option<Hold>> }
impl std::ops::Deref for Slots {
    type Target = [Option<Binding>];
    fn deref(&self) -> &Self::Target { &self.bindings }
}
impl Slots {
    /// Puts `binding` in slot `at` and gives back the binding it replaces.
    fn set(&mut self, at: usize, binding: Option<Binding>) -> Option<Binding> {
        self.holds[at] = binding.as_ref().and_then(held);
        std::mem::replace(&mut self.bindings[at], binding)
    }
    fn resize(&mut self, len: usize) {
        self.bindings.resize_with(len, || None);
        self.holds.resize_with(len, || None);
    }
    fn push(&mut self) -> usize { self.resize(self.len() + 1); self.len() - 1 }
}
/// A binding's hold on its value, where the value is pending.
fn held(binding: &Binding) -> Option<Hold> { if let Binding::Value(value) = binding { Hold::of(value) } else { None } }
struct Chain<'a> { frames: &'a [Frame], scope: Option<usize> }
impl<'a> Iterator for Chain<'a> {
    type Item = (usize, &'a Frame);
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.scope?;
        let frame = self.frames.get(i)?;
        self.scope = frame.parent;
        Some((i, frame))
    }
}

struct Module { id: u64, entries: Slots, index: HashMap<Name, u32> }
impl Module {
    fn new() -> Self { Self { id: IDS.fetch_add(1, Relaxed), entries: Slots::default(), index: HashMap::new() } }
    fn get(&self, name: Name) -> Option<(u32, &Binding)> { let at = *self.index.get(&name)?; Some((at, self.entries[at as usize].as_ref()?)) }
    fn entry(&mut self, name: Name) -> u32 {
        let Self { entries, index, .. } = self;
        *index.entry(name).or_insert_with(|| entries.push() as u32)
    }
    fn bound(&self) -> impl Iterator<Item = (Name, &Binding)> { self.index.iter().filter_map(|(&name, &at)| Some((name, self.entries[at as usize].as_ref()?))) }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Address { Implicit(usize), Free(Box<Free>) }
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Free {
    name: Name,
    // A captured slot stops lookup even when its call-time binding was absent.
    slots: Box<[(u32, Option<u32>, bool)]>,
    module: usize,
    id: u64,
    entry: Option<u32>,
}

pub(super) struct Bindings { stack: RefCell<Stack>, modules: RefCell<Vec<Module>>, current: Cell<Option<usize>>, module: Cell<usize> }
impl Default for Bindings {
    fn default() -> Self {
        Self { stack: RefCell::default(), modules: RefCell::new(vec![Module::new()]), current: Cell::default(), module: Cell::default() }
    }
}
impl Bindings {
    fn with_origin<R>(&self, inputs: &FrameInputs, run: impl FnOnce(crate::program::Origin<&[Value]>) -> crate::error::Result<R>) -> crate::error::Result<R> {
        let stack = self.stack.borrow();
        let frame =
            stack.frames.get(inputs.frame).filter(|frame| frame.id == inputs.generation).ok_or_else(|| Error::domain("function's call frame has ended"))?;
        if stack.slots[frame.captured()..frame.slots().end].iter().all(Option::is_none) {
            return run(frame.origin.as_ref().expect("a captured binding has call context").reborrow());
        }
        drop(stack);
        let origin = self.origin_at(inputs, None)?;
        run(origin.reborrow())
    }
    fn origin_at(&self, inputs: &FrameInputs, source: Option<crate::program::Origin<&[Value]>>) -> crate::error::Result<crate::program::Origin<Vec<Value>>> {
        let stack = self.stack.borrow();
        let frame =
            stack.frames.get(inputs.frame).filter(|frame| frame.id == inputs.generation).ok_or_else(|| Error::domain("function's call frame has ended"))?;
        let source = source.unwrap_or_else(|| frame.origin.as_ref().expect("a captured binding has call context").reborrow());
        let overrides = &stack.slots[frame.captured()..frame.slots().end];
        if overrides.iter().all(Option::is_none) { return Ok(source.with_list(source.list.to_vec())); }
        let mut result = super::map_roots(&source, |root| {
            let root = root.index.get();
            match &overrides[root] {
                Some(Binding::Value(value)) => Ok(Some(super::RootReplacement::value(value))),
                None | Some(Binding::Absent | Binding::Released) => Ok(None), _ => Err(Error::domain("a captured override must be a produced value"))
            }
        })?;
        for (old, scope) in source.bindings.scopes.iter().zip(&mut std::sync::Arc::make_mut(&mut result.bindings).scopes) {
            for (old, name) in old.names.iter().zip(&mut scope.names) {
                if old.position.as_ref().and_then(|position| frame.root_slot(position))
                    .is_some_and(|slot| matches!(stack.slots[slot], Some(Binding::Absent | Binding::Released))) { name.position = None; }
            }
        }
        Ok(result)
    }

    pub(super) fn active_origin(&self, source: &crate::program::Origin<&[Value]>) -> crate::error::Result<Option<crate::program::Origin<Vec<Value>>>> {
        let inputs = {
            let stack = self.stack.borrow();
            stack.frames.iter().enumerate().rev().find_map(|(i, frame)| {
                let original = frame.origin.as_ref()?;
                (std::sync::Arc::ptr_eq(&original.bindings, &source.bindings) && stack.slots[frame.captured()..frame.slots().end].iter().any(Option::is_some))
                    .then(|| frame.inputs(i))
            })
        };
        inputs.map(|inputs| self.origin_at(&inputs, Some(source.reborrow()))).transpose()
    }

    pub(super) fn replace(&self, other: Bindings) {
        self.stack.replace(other.stack.into_inner());
        self.modules.replace(other.modules.into_inner());
        self.current.set(other.current.get());
        self.module.set(other.module.get());
    }
    pub(super) fn current(&self) -> Option<usize> { self.current.get() }
    pub(super) fn inputs(&self) -> Option<FrameInputs> {
        let current = self.current.get()?;
        let stack = self.stack.borrow();
        let frame = &stack.frames[current];
        frame.origin.as_ref().map(|_| frame.inputs(current))
    }
    pub(super) fn module(&self) -> usize { self.current.get().map_or(self.module.get(), |i| self.stack.borrow().frames[i].module) }
    pub(super) fn capture_context(&self, parent: Option<usize>) -> Option<super::CaptureScope> {
        let stack = self.stack.borrow();
        Chain { frames: &stack.frames, scope: parent }.find_map(|(_, frame)| frame.closed().cloned())
    }
    fn chain<'a>(&self, frames: &'a [Frame]) -> Chain<'a> { Chain { frames, scope: self.current.get() } }
    pub(super) fn here(&self, name: Name) -> Place {
        match self.current.get() {
            Some(i) => self.stack.borrow().frames[i].place(i, name),
            None => { let m = self.module.get(); Place::Entry(m, self.modules.borrow_mut()[m].entry(name)) }
        }
    }
    pub(super) fn local(&self, slot: u32) -> Place {
        let i = self.current.get().expect("a local has a running frame");
        Place::Slot(i, slot, self.stack.borrow().frames[i].id)
    }
    pub(super) fn lookup(&self, name: Name) -> Result<Option<(Place, Binding)>, Error> { self.find(name, |place, value| (place, value.clone())) }
    pub(super) fn category(&self, name: Name) -> Result<Option<Category>, Error> { self.find(name, |_, value| Category::of(value)) }
    pub(super) fn role(&self, name: Name) -> Result<Option<(Category, bool)>, Error> { self.find(name, |_, value| super::binder::role(value)) }
    fn find<T>(&self, name: Name, read: impl FnOnce(Place, &Binding) -> T) -> Result<Option<T>, Error> {
        self.find_from(self.current.get(), self.module(), name, read)
    }
    fn find_from<T>(&self, parent: Option<usize>, module: usize, name: Name, read: impl FnOnce(Place, &Binding) -> T) -> Result<Option<T>, Error> {
        let stack = self.stack.borrow();
        for (i, frame) in (Chain { frames: &stack.frames, scope: parent }) {
            if let Some(slot) = frame.slot(name) && let Some(value) = &stack.slots[frame.base + slot as usize] {
                return Ok(Some(read(Place::Slot(i, slot, frame.id), value)));
            }
            if let Some(value) = frame.other(name) { return Ok(Some(read(Place::Other(i, name, frame.id), value))); }
            if let Some(value) = frame.key(name)? { return Ok(Some(read(Place::Other(i, name, frame.id), &value))); }
            if let Some(captured) = frame.closed().and_then(|scope| scope.names.iter().find(|binding| binding.name == name)) {
                let Some(position) = &captured.position else { return Ok(None) };
                let slot = frame.root_slot(position).or_internal()?;
                return Ok(frame.root_binding(i, slot, &stack.slots)?
                    .map(|binding| read(captured.place.unwrap_or(Place::Slot(i, (slot - frame.base) as u32, frame.id)), &binding)));
            }
        }
        if name.implicit().is_some() { return Ok(None); }
        drop(stack);
        Ok(self.modules.borrow()[module].get(name).map(|(at, value)| read(Place::Entry(module, at), value)))
    }
    pub(super) fn capture(&self, parent: Option<usize>, module: usize, name: Name) -> Result<Option<(Place, Binding)>, Error> {
        let stack = self.stack.borrow();
        if let Some(i) = parent && let Some(frame) = stack.frames.get(i) {
            if let Some(slot) = frame.slot(name) && let Some(value) = &stack.slots[frame.base + slot as usize] {
                return Ok(Some((Place::Slot(i, slot, frame.id), value.clone())));
            }
            if let Some(value) = frame.other(name) { return Ok(Some((Place::Other(i, name, frame.id), value.clone()))); }
            if let Some(value) = frame.key(name)? { return Ok(Some((Place::Other(i, name, frame.id), value))); }
        }
        for (i, frame) in stack.frames.iter().enumerate().rev() {
            let Some(origin) = &frame.origin else { continue };
            for scope in &origin.bindings.scopes {
                if scope.parent != parent || scope.module != module { continue; }
                if let Some(captured) = scope.names.iter().find(|binding| binding.name == name) {
                    let Some(position) = &captured.position else { return Ok(None) };
                    let slot = frame.root_slot(position).or_internal()?;
                    return Ok(frame.root_binding(i, slot, &stack.slots)?
                        .map(|binding| (captured.place.unwrap_or(Place::Slot(i, (slot - frame.base) as u32, frame.id)), binding)));
                }
            }
        }
        drop(stack);
        self.find_from(parent, module, name, |place, value| (place, value.clone()))
    }
    pub(super) fn get(&self, place: Place) -> Result<Option<Binding>, Error> {
        let stack = self.stack.borrow();
        for (i, frame) in stack.frames.iter().enumerate().rev() {
            if let Some(origin) = &frame.origin {
                for captured in origin.bindings.scopes.iter().flat_map(|scope| scope.names.iter()) {
                    if captured.place == Some(place) {
                        let Some(position) = &captured.position else { return Ok(None) };
                        return frame.root_binding(i, frame.root_slot(position).or_internal()?, &stack.slots);
                    }
                }
            }
        }
        Ok(match place {
            Place::Slot(i, slot, id) => stack.frames.get(i).filter(|frame| frame.id == id).and_then(|frame| stack.slots[frame.base + slot as usize].clone()),
            Place::Other(i, name, id) => {
                let Some(frame) = stack.frames.get(i).filter(|frame| frame.id == id) else { return Ok(None) };
                if let Some(binding) = frame.other(name) { Some(binding.clone()) } else { frame.key(name)? }
            }
            Place::Entry(module, entry) => self.modules.borrow().get(module).and_then(|module| module.entries.get(entry as usize)).cloned().flatten(),
        })
    }
    fn aliases(stack: &mut Stack, place: Place, value: Option<&Binding>) {
        for frame in &stack.frames {
            let Some(origin) = &frame.origin else { continue };
            for captured in origin.bindings.scopes.iter().flat_map(|scope| scope.names.iter()) {
                if captured.place == Some(place) && let Some(slot) = captured.position.as_ref().and_then(|position| frame.root_slot(position)) {
                    stack.slots.set(slot, value.map(|binding| match binding {
                        Binding::Function(super::FunctionBinding::Input(function)) => Binding::Value(Value::Function(function.clone())),
                        Binding::Operator(operator) => Binding::Value(Value::Operator(std::sync::Arc::new(operator.clone()))), binding => binding.clone()
                    }).or(Some(Binding::Absent)));
                }
            }
        }
    }
    pub(super) fn insert(&self, place: Place, value: Binding) {
        let mut stack = self.stack.borrow_mut();
        match place {
            Place::Slot(i, slot, id) => {
                if let Some(frame) = stack.frames.get(i).filter(|frame| frame.id == id) {
                    let at = frame.base + slot as usize;
                    stack.slots.set(at, Some(value.clone()));
                }
            }
            Place::Other(i, name, id) => {
                if let Some(frame) = stack.frames.get_mut(i).filter(|frame| frame.id == id) {
                    frame.unslotted.get_or_insert_default().names.insert(name, (value.clone(), held(&value)));
                }
            }
            Place::Entry(module, entry) => { self.modules.borrow_mut()[module].entries.set(entry as usize, Some(value.clone())); }
        }
        Self::aliases(&mut stack, place, Some(&value));
    }
    pub(super) fn remove(&self, place: Place) -> Option<Binding> {
        let mut stack = self.stack.borrow_mut();
        let old = match place {
            Place::Slot(i, slot, id) => {
                let frame = stack.frames.get(i)?;
                if frame.id != id { return None; }
                let at = frame.base + slot as usize;
                stack.slots.set(at, None)
            }
            Place::Other(i, name, id) => {
                let frame = stack.frames.get_mut(i)?;
                if frame.id != id { return None; }
                frame.unslotted.as_mut()?.names.remove(&name).map(|(binding, _)| binding)
            }
            Place::Entry(module, entry) => self.modules.borrow_mut()[module].entries.set(entry as usize, None),
        };
        Self::aliases(&mut stack, place, None);
        old
    }
    pub(super) fn address(&self, name: Name) -> Address {
        if let Some(k) = name.implicit() { return Address::Implicit(k); }
        let stack = self.stack.borrow();
        let mut slots = Vec::new();
        for (depth, (_, frame)) in self.chain(&stack.frames).enumerate() {
            if let Some(slot) = frame.slot(name) { slots.push((depth as u32, Some(slot), false)); }
            if let Some(captured) = frame.closed().and_then(|scope| scope.names.iter().find(|binding| binding.name == name)) {
                let slot = captured.position.as_ref().and_then(|position| frame.root_slot(position)).map(|slot| (slot - frame.base) as u32);
                slots.push((depth as u32, slot, true));
                break;
            }
        }
        let module = self.module();
        let modules = self.modules.borrow();
        Address::Free(Box::new(Free { name, slots: slots.into(), module, id: modules[module].id, entry: modules[module].index.get(&name).copied() }))
    }
    pub(super) fn load(&self, address: &Address) -> Result<Option<Binding>, Error> {
        match self.read_at(address, Binding::clone)? { Some(binding) => Ok(binding), None => self.unaddressed(address, Binding::clone) }
    }
    fn own_slot(address: &Address) -> Option<u32> {
        match address {
            Address::Implicit(k) => Some(*k as u32), Address::Free(free) => match free.slots.first() { Some(&(0, Some(slot), false)) => Some(slot), _ => None }
        }
    }
    pub(super) fn take_at(&self, address: &Address) -> Option<Binding> {
        let slot = Self::own_slot(address)?;
        let i = self.current.get()?;
        let mut stack = self.stack.borrow_mut();
        let at = stack.frames[i].base + slot as usize;
        stack.slots[at].as_ref()?;
        stack.slots.set(at, Some(Binding::Released))
    }
    pub(super) fn role_at(&self, address: &Address) -> Result<Option<(Category, bool)>, Error> {
        match self.read_at(address, super::binder::role)? { Some(role) => Ok(role), None => self.unaddressed(address, super::binder::role) }
    }
    fn read_at<R>(&self, address: &Address, read: impl Fn(&Binding) -> R) -> Result<Option<Option<R>>, Error> {
        let stack = self.stack.borrow();
        let Address::Free(free) = address else {
            let Address::Implicit(k) = address else { unreachable!() };
            return Ok(Some(self.chain(&stack.frames).find_map(|(_, frame)| stack.slots[frame.base + k].as_ref().map(&read))));
        };
        let mut candidates = free.slots.iter().peekable();
        for (depth, (frame_index, frame)) in self.chain(&stack.frames).enumerate() {
            if frame.unslotted.is_some() { return Ok(None); }
            while let Some(&&(at_depth, slot, stop)) = candidates.peek() {
                if at_depth as usize != depth { break; }
                candidates.next();
                let value = match slot {
                    Some(slot) if stop => frame.root_binding(frame_index, frame.base + slot as usize, &stack.slots)?,
                    Some(slot) => stack.slots[frame.base + slot as usize].clone(), None => None
                };
                if value.is_some() || stop { return Ok(Some(value.as_ref().map(&read))); }
            }
        }
        drop(stack);
        let modules = self.modules.borrow();
        let Some(module) = modules.get(free.module).filter(|module| module.id == free.id) else { return Ok(None) };
        Ok(free.entry.map(|entry| module.entries[entry as usize].as_ref().map(read)))
    }
    fn unaddressed<R>(&self, address: &Address, read: impl Fn(&Binding) -> R) -> Result<Option<R>, Error> {
        let Address::Free(free) = address else { unreachable!() };
        self.find(free.name, |_, value| read(value))
    }
    pub(super) fn names(&self) -> Vec<String> {
        let stack = self.stack.borrow();
        let mut names = HashSet::new();
        for (_, frame) in self.chain(&stack.frames) { names.extend(frame.names(&stack.slots).map(str::to_owned)); }
        names.extend(self.modules.borrow()[self.module()].bound().map(|(name, _)| name.text().to_owned()));
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort();
        names
    }
    pub(super) fn new_module(&self) -> usize {
        let mut modules = self.modules.borrow_mut();
        modules.push(Module::new());
        modules.len() - 1
    }
    pub(super) fn module_names(&self, module: usize) -> Vec<(Name, Binding)> {
        let mut names: Vec<_> = self.modules.borrow()[module].bound().map(|(name, value)| (name, value.clone())).collect();
        names.sort_by_key(|&(name, _)| name);
        names
    }
    pub(super) fn at_top_level<T>(&self, module: Option<usize>, run: impl FnOnce() -> T) -> T {
        let (current, running) = (self.current.replace(None), self.module.get());
        if let Some(module) = module { self.module.set(module); }
        let result = run();
        self.current.set(current);
        self.module.set(running);
        result
    }
    pub(super) fn depth(&self) -> usize { self.stack.borrow().frames.len() }
    pub(super) fn enter(
        &self, mut frame: Frame, mut implicit: [Option<Binding>; IMPLICIT], closed: Option<(Option<u32>, crate::program::Origin<&[Value]>)>, span: &Span
    ) -> Result<(), Error> {
        if let Some((scope, origin)) = closed {
            frame.scope = scope;
            frame.roots = origin.trees.as_ref()
                .map_or_else(|| (0..origin.list.len()).map(|index| TreeRoot::at(None, Root(index as u32))).collect(), |trees| trees.roots().collect());
            frame.origin = Some(origin.with_list(origin.list.to_vec()));
        }
        let mut stack = self.stack.borrow_mut();
        if stack.frames.len() == MAX_FRAME_DEPTH { return Err(Error::new(Kind::Limit, "too many active frames").at(span)); }
        frame.base = stack.slots.len();
        let inputs = frame.inputs(stack.frames.len());
        for binding in implicit.iter_mut().flatten() {
            if let Binding::Function(super::FunctionBinding::Input(function)) = binding
                && frame.origin.as_ref().is_some_and(|origin| origin.bindings.scope(function).is_some()) {
                    *binding = Binding::Function(super::FunctionBinding::Rebuild { template: function.clone(), dependencies: inputs.clone() });
                }
        }
        stack.slots.resize(frame.slots().end);
        for (k, binding) in implicit.into_iter().enumerate() { stack.slots.set(frame.base + k, binding); }
        self.current.set(Some(stack.frames.len()));
        stack.frames.push(frame);
        Ok(())
    }
    pub(super) fn enter_context(&self, origin: crate::program::Origin<&[Value]>, span: &Span) -> Result<(), Error> {
        let current = self.current.get();
        self.enter(Frame::new(None, self.module(), &[]), Default::default(), Some((None, origin)), span)?;
        self.current.set(current);
        Ok(())
    }
    pub(super) fn truncate(&self, depth: usize) {
        let mut stack = self.stack.borrow_mut();
        if let Some(base) = stack.frames.get(depth).map(|frame| frame.base) { stack.slots.resize(base); }
        stack.frames.truncate(depth);
    }
    pub(super) fn leave(&self, depth: usize, current: Option<usize>) {
        self.truncate(depth);
        self.current.set(current);
    }
}
