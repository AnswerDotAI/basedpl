//! Binding storage: the frames of running calls and scopes, each module's names, and where code is running. Code finds where a name is bound as a `Place`, and reads and writes bindings only through `Bindings`.

use super::{Binding, Category, MAX_FRAME_DEPTH};
use crate::{name::Name, Error, Span, Value};
use rustymath::{error::Kind, hashing::{HashMap, HashMapExt, HashSet, HashSetExt}};
use std::{cell::{Cell, RefCell}, ops::Range, sync::atomic::{AtomicU64, Ordering::Relaxed}};

/// Where a binding is, or where an assignment puts one: a frame's slot, by the frame's index and the slot's number among the frame's slots; a name that a frame binds without a slot, such as a key of a scope's record; or an entry in a module's names, by the module's number and the entry's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place { Slot(usize, u32), Other(usize, Name), Entry(usize, u32) }

impl Place {
    /// The frame that holds the binding, or `None` for a module's names, which outlive every frame. A binding can't hold a value that refers to a later frame.
    pub(crate) fn frame(self) -> Option<usize> { match self { Self::Slot(i, _) | Self::Other(i, _) => Some(i), Self::Entry(..) => None } }
}

/// The number of slots before a frame's locals: one for each of the six names that a call binds itself, in the order of `Name::implicit`.
const IMPLICIT: usize = 6;

/// What a frame binds without slots: names that `⍎` binds where the frame's definition has no slot for them, `$e`, a scope's assignments, and a scope's record, whose keys on its first axis name its items or major cells (`Frame::key`). A key's text never becomes a `Name`, so a record's keys never add to the process's names.
#[derive(Clone, Default)]
struct Unslotted { names: HashMap<Name, Binding>, record: Option<Value> }

/// The frame of a running call or scope. Its slots on the slot stack hold the six names that a call binds itself, then the names that its definition gives slots. It also has what it binds without slots, the frame it sees names through after its own (`parent`), and the module whose names it sees after its frames.
pub(super) struct Frame {
    /// The names that have slots after the six that a call binds itself, in slot order: the definition's locals (`Definition::locals`). Empty for a scope, and for a definition that assigns no names.
    locals: &'static [Name],
    /// Where the frame's slots start on the slot stack.
    base: usize,
    /// What the frame binds without slots. Most frames of calls bind nothing there and allocate nothing for it.
    unslotted: Option<Box<Unslotted>>,
    parent: Option<usize>,
    module: usize,
}

impl Frame {
    /// The frame of a call, with nothing bound yet. `locals` are the names that its definition assigns.
    #[inline]
    pub(super) fn new(parent: Option<usize>, module: usize, locals: &'static [Name]) -> Self { Self { locals, base: 0, unslotted: None, parent, module } }

    /// The frame of a scope over `record`, which has keys on its first axis, with nothing bound yet.
    pub(super) fn scope(parent: Option<usize>, module: usize, record: Value) -> Self { Self { unslotted: Some(Box::new(Unslotted { names: HashMap::new(), record: Some(record) })), ..Self::new(parent, module, &[]) } }

    /// Whether the frame binds names without slots: names that `⍎` or an assignment binds where the frame has no slot for them, or a scope's keys.
    #[inline]
    fn unslotted(&self) -> bool { self.unslotted.is_some() }

    /// Whether the frame can bind a name other than the six that a call binds itself.
    #[inline]
    fn binds_named(&self) -> bool { !self.locals.is_empty() || self.unslotted() }

    /// The positions of the frame's slots on the slot stack.
    #[inline]
    fn slots(&self) -> Range<usize> { self.base..self.base + IMPLICIT + self.locals.len() }

    /// The number among the frame's slots of `name`'s slot, where the frame has one.
    #[inline]
    fn slot(&self, name: Name) -> Option<u32> {
        match name.implicit() { Some(k) => Some(k as u32), None => self.locals.binary_search(&name).ok().map(|k| (IMPLICIT + k) as u32) }
    }

    /// The binding of `name` among the names that the frame binds without slots.
    #[inline]
    fn other(&self, name: Name) -> Option<&Binding> { self.unslotted.as_ref()?.names.get(&name) }

    /// The binding of `name`, a name other than the six that a call binds itself, in frame `i`, with its place, besides a key of a scope's record (`key`). A name that has a slot is bound there or nowhere in the frame. It's out of line, so that `Bindings::find` stays small, and `find` calls it only for a frame that `binds_named`.
    #[inline(never)]
    fn named<'a>(&'a self, i: usize, slots: &'a [Option<Binding>], name: Name) -> Option<(Place, &'a Binding)> {
        match self.slot(name) { Some(s) => Some((Place::Slot(i, s), slots[self.base + s as usize].as_ref()?)), None => Some((Place::Other(i, name), self.other(name)?)) }
    }

    /// The item or major cell that `name` names as a key on the first axis of the frame's record, where the frame is a scope's. Only a scope's frame reads a name's text.
    fn key(&self, name: Name) -> Option<Binding> {
        let record = self.unslotted.as_ref()?.record.as_ref()?;
        let at = record.keys(0)?.position(name.text())?;
        let item = if record.shape().len() == 1 { record.at(at) } else { record.cells(record.shape().len() - 1).ok()?.get(at).ok()? };
        Some(Binding::from_element(item))
    }

    /// The text of each name that the frame binds, besides the six that a call binds itself.
    fn names<'a>(&'a self, slots: &'a [Option<Binding>]) -> impl Iterator<Item = &'a str> {
        let locals = self.locals.iter().zip(&slots[self.base + IMPLICIT..]).filter(|(_, slot)| slot.is_some()).map(|(name, _)| name.text());
        let keys = |record: &'a Value| record.keys(0).into_iter().flat_map(|keys| keys.names().iter().flatten().map(|key| &**key));
        locals.chain(self.unslotted.iter().flat_map(move |u| u.names.keys().map(|name| name.text()).chain(u.record.iter().flat_map(keys))))
    }
}

/// A copy of a frame's bindings, which a guard case reads at its cells and then swaps in (`Session::restricted`).
pub(super) struct FrameBindings { slots: Vec<Option<Binding>>, unslotted: Option<Box<Unslotted>> }

impl FrameBindings {
    /// Each array that the frame binds, and a scope's record, for a guard case to read at its cells.
    pub(super) fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        let (names, record) = match self.unslotted.as_deref_mut() { Some(Unslotted { names, record }) => (Some(names), record.as_mut()), None => (None, None) };
        let bindings = self.slots.iter_mut().flatten().chain(names.into_iter().flat_map(|names| names.values_mut()));
        bindings.filter_map(|binding| match binding { Binding::Value(v) => Some(v), _ => None }).chain(record)
    }
}

/// The frames of running calls and scopes, innermost last, and the slot stack, which holds each frame's slots after those of the frames below it. A call pushes its slots there and allocates nothing for its names.
#[derive(Default)]
struct Stack { frames: Vec<Frame>, slots: Vec<Option<Binding>> }

/// The frames that code sees names through from frame `scope`, with their indices: that frame, then each frame's parent in turn (`Bindings::chain`).
struct Chain<'f> { frames: &'f [Frame], scope: Option<usize> }

impl<'f> Iterator for Chain<'f> {
    type Item = (usize, &'f Frame);
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.scope?;
        let frame = &self.frames[i];
        self.scope = frame.parent;
        Some((i, frame))
    }
}

/// A module's names. `index` gives each name's entry, which holds its binding. Deleting a name empties its entry, which keeps its place, so the entry number in a compiled load (`Free`) stays the name's. Every reader of a module's names skips empty entries.
struct Module {
    /// A number that no other module in the process has. A recording that holds a module's entry numbers can run in another session, where the module of the same number is a different one.
    id: u64,
    entries: Vec<Option<Binding>>,
    index: HashMap<Name, u32>,
}

/// The `id` that the next module takes.
static MODULE_IDS: AtomicU64 = AtomicU64::new(0);

impl Module {
    fn new() -> Self { Self { id: MODULE_IDS.fetch_add(1, Relaxed), entries: Vec::new(), index: HashMap::new() } }
    /// The binding of `name`, with its entry number.
    #[inline]
    fn get(&self, name: Name) -> Option<(u32, &Binding)> { let at = *self.index.get(&name)?; Some((at, self.entries[at as usize].as_ref()?)) }
    /// `name`'s entry number, which a new empty entry takes where the module has none.
    fn entry(&mut self, name: Name) -> u32 {
        let Self { entries, index, .. } = self;
        *index.entry(name).or_insert_with(|| { entries.push(None); entries.len() as u32 - 1 })
    }
    /// Each name that the module binds, with its binding.
    fn bound(&self) -> impl Iterator<Item = (Name, &Binding)> { self.index.iter().filter_map(|(&name, &at)| Some((name, self.entries[at as usize].as_ref()?))) }
}

/// Where a compiled load reads a name (`Bindings::load`), worked out when the binder records the read (`Bindings::address`).
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Address {
    /// One of the six names that a call binds itself, by its slot (`Name::implicit`).
    Implicit(usize),
    Free(Box<Free>),
}

/// Where a load finds any other name: the slots on the lexical chain that can hold it, nearest first, then the running module's entry for it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Free {
    name: Name,
    /// Each frame on the chain whose definition gives `name` a slot, as its depth on the chain and the slot's number among its slots.
    slots: Box<[(u32, u32)]>,
    /// The running module's number, and its `id`.
    module: usize,
    id: u64,
    /// `name`'s entry in the module, where the module had one when the read was recorded.
    entry: Option<u32>,
}

/// Every binding, and where code is running: the frames of running calls and scopes with their slots, each module's names, the current frame, and the module whose top level is running.
pub(super) struct Bindings {
    stack: RefCell<Stack>,
    /// The names of each module, by module number. Module 0 holds the session's own names, and each loaded file adds a module.
    modules: RefCell<Vec<Module>>,
    /// The frame that code is running in, or `None` at a module's top level.
    current: Cell<Option<usize>>,
    /// The module whose top level is running.
    module: Cell<usize>,
}

impl Default for Bindings {
    fn default() -> Self { Self { stack: RefCell::default(), modules: RefCell::new(vec![Module::new()]), current: Cell::default(), module: Cell::default() } }
}

impl Bindings {
    /// Takes `other`'s bindings in place of these, for `Session::clear`.
    pub(super) fn replace(&self, other: Bindings) {
        let Bindings { stack, modules, current, module } = other;
        self.stack.replace(stack.into_inner());
        self.modules.replace(modules.into_inner());
        self.current.set(current.get());
        self.module.set(module.get());
    }

    /// The frame that code is running in, or `None` at a module's top level.
    #[inline]
    pub(super) fn current(&self) -> Option<usize> { self.current.get() }

    /// The module whose names code sees after its frames: the module that defined the running function, or the module whose top level is running.
    #[inline]
    pub(super) fn module(&self) -> usize { self.running_module(&self.stack.borrow().frames) }

    /// `module`, read from `frames`, which the caller holds borrowed.
    #[inline]
    fn running_module(&self, frames: &[Frame]) -> usize { self.current.get().map_or(self.module.get(), |i| frames[i].module) }

    /// The frames that code here sees names through, with their indices: the current frame, then each frame's parent in turn.
    #[inline(always)]
    fn chain<'f>(&self, frames: &'f [Frame]) -> Chain<'f> { Chain { frames, scope: self.current.get() } }

    /// Where a plain assignment binds `name` here: the current frame's slot for it, or its other names where it has none, or the running module's entry for it at a module's top level.
    #[inline]
    pub(super) fn here(&self, name: Name) -> Place {
        match self.current.get() {
            Some(i) => self.stack.borrow().frames[i].slot(name).map_or(Place::Other(i, name), |s| Place::Slot(i, s)),
            None => { let m = self.module.get(); Place::Entry(m, self.modules.borrow_mut()[m].entry(name)) }
        }
    }

    /// The binding of `name` that code sees here, copied, with its place. `None` where nothing binds `name`.
    #[inline]
    pub(super) fn lookup(&self, name: Name) -> Option<(Place, Binding)> { self.find(name, |place, value| (place, value.clone())) }

    /// The category of the binding of `name` that code sees here, for callers that only test what kind of binding a name has. It copies no binding, so it updates no reference count.
    #[inline]
    pub(super) fn category(&self, name: Name) -> Option<Category> { self.find(name, |_, value| Category::of(value)) }

    /// The binding that code sees here of the name that a call binds itself at slot `k`, with the frame that holds it.
    #[inline(always)]
    fn implicit<'s>(&self, frames: &'s [Frame], slots: &'s [Option<Binding>], k: usize) -> Option<(usize, &'s Binding)> {
        for (i, frame) in self.chain(frames) { if let Some(value) = &slots[frame.base + k] { return Some((i, value)); } }
        None
    }

    /// `f` on the binding of `name` that code sees here, and its place. Lookup tries the current frame, then each frame that it sees names through, then the running module's names. Every call's frame binds `⍺`, to an absent argument where the call has no left argument, so lookup finds the current call's `⍺`. No module holds the six names that a call binds itself. In each frame, a name that has a slot is in its slot or unbound there, and an empty slot sends the lookup on to the next frame, as an unbound name does. In a scope's frame, a key of its record names its item or major cell, after any name that an assignment in the scope binds. The binding stays borrowed while `f` reads it, so `f` evaluates nothing.
    fn find<T>(&self, name: Name, f: impl FnOnce(Place, &Binding) -> T) -> Option<T> {
        let stack = self.stack.borrow();
        let Stack { frames, slots } = &*stack;
        if let Some(k) = name.implicit() { return self.implicit(frames, slots, k).map(|(i, value)| f(Place::Slot(i, k as u32), value)); }
        for (i, frame) in self.chain(frames) {
            if !frame.binds_named() { continue; }
            if let Some((place, value)) = frame.named(i, slots, name) { return Some(f(place, value)); }
            if let Some(value) = frame.key(name) { return Some(f(Place::Other(i, name), &value)); }
        }
        let module = self.running_module(frames);
        drop(stack);
        self.modules.borrow()[module].get(name).map(|(at, value)| f(Place::Entry(module, at), value))
    }

    /// The binding at `place`, copied. `None` where nothing is bound there, or `place` is in a frame that has ended.
    #[inline]
    pub(super) fn get(&self, place: Place) -> Option<Binding> {
        match place {
            Place::Slot(i, s) => { let stack = self.stack.borrow(); stack.slots[stack.frames.get(i)?.base + s as usize].clone() }
            Place::Other(i, name) => { let stack = self.stack.borrow(); let frame = stack.frames.get(i)?; frame.other(name).cloned().or_else(|| frame.key(name)) }
            Place::Entry(m, at) => self.modules.borrow()[m].entries[at as usize].clone(),
        }
    }

    /// Binds `place` to `value`.
    #[inline]
    pub(super) fn insert(&self, place: Place, value: Binding) {
        match place {
            Place::Slot(i, s) => { let mut stack = self.stack.borrow_mut(); let at = stack.frames[i].base + s as usize; stack.slots[at] = Some(value); }
            Place::Other(i, name) => { self.stack.borrow_mut().frames[i].unslotted.get_or_insert_default().names.insert(name, value); }
            Place::Entry(m, at) => self.modules.borrow_mut()[m].entries[at as usize] = Some(value),
        }
    }

    /// Removes the binding at `place`, and gives it back. A slot or entry stays, empty.
    pub(super) fn remove(&self, place: Place) -> Option<Binding> {
        match place {
            Place::Slot(i, s) => { let mut stack = self.stack.borrow_mut(); let at = stack.frames[i].base + s as usize; stack.slots[at].take() }
            Place::Other(i, name) => self.stack.borrow_mut().frames[i].unslotted.as_mut()?.names.remove(&name),
            Place::Entry(m, at) => self.modules.borrow_mut()[m].entries[at as usize].take(),
        }
    }

    /// The address of `name` for a compiled load, read from the frames and module that code runs in now. A dfn's lexical chain is the same on every call, because a closure's environment is a frame of the definition around its literal, so the address holds while no frame on the chain binds names without slots (`load`).
    pub(super) fn address(&self, name: Name) -> Address {
        if let Some(k) = name.implicit() { return Address::Implicit(k); }
        let stack = self.stack.borrow();
        let slots = self.chain(&stack.frames).enumerate().filter_map(|(depth, (_, frame))| Some((depth as u32, frame.slot(name)?))).collect();
        let module = self.running_module(&stack.frames);
        let modules = self.modules.borrow();
        Address::Free(Box::new(Free { name, slots, module, id: modules[module].id, entry: modules[module].index.get(&name).copied() }))
    }

    /// The binding at `address`, copied, as `lookup` finds it. `None` where nothing binds the name.
    #[inline(always)]
    pub(super) fn load(&self, address: &Address) -> Option<Binding> { self.read_at(address, Binding::clone).unwrap_or_else(|| self.unaddressed(address, Binding::clone)) }

    /// The category of the binding at `address`, as `category` gives it, for a recording's check of a fact.
    pub(super) fn category_at(&self, address: &Address) -> Option<Category> { self.read_at(address, Category::of).unwrap_or_else(|| self.unaddressed(address, Category::of)) }

    /// `read` on the binding at `address`, as `find` finds it, or `Some(None)` where nothing binds the name. `None` where the address can't tell: a frame on the chain binds names without slots, as `⍎` and scopes bind them, the running module isn't the recorded one, or the module had no entry for the name when the address was worked out.
    #[inline(always)]
    fn read_at<R>(&self, address: &Address, read: impl Fn(&Binding) -> R) -> Option<Option<R>> {
        let stack = self.stack.borrow();
        let Stack { frames, slots } = &*stack;
        let free = match address { Address::Implicit(k) => return Some(self.implicit(frames, slots, *k).map(|(_, value)| read(value))), Address::Free(free) => free };
        let mut shadows = free.slots.iter().peekable();
        for (depth, (_, frame)) in self.chain(frames).enumerate() {
            if frame.unslotted() { return None; }
            if let Some(&&(at, s)) = shadows.peek() && at as usize == depth {
                shadows.next();
                if let Some(value) = &slots[frame.base + s as usize] { return Some(Some(read(value))); }
            }
        }
        if self.running_module(frames) != free.module { return None; }
        let modules = self.modules.borrow();
        let module = &modules[free.module];
        if module.id != free.id { return None; }
        Some(module.entries[free.entry? as usize].as_ref().map(read))
    }

    /// `read` on the binding of the name at `address`, looked up by `find`, where the address can't tell (`read_at`). Out of line, so that a compiled load stays small.
    #[cold]
    #[inline(never)]
    fn unaddressed<R>(&self, address: &Address, read: impl Fn(&Binding) -> R) -> Option<R> {
        let Address::Free(free) = address else { unreachable!("the address of a name that a call binds itself always tells") };
        self.find(free.name, |_, value| read(value))
    }

    /// The text of each name that code sees here, once each, sorted: the names of the current frame and of each frame that it sees names through, then the running module's. The six names that a call binds itself, and empty slots, aren't listed.
    pub(super) fn names(&self) -> Vec<String> {
        let stack = self.stack.borrow();
        let mut names = HashSet::new();
        for (_, frame) in self.chain(&stack.frames) { names.extend(frame.names(&stack.slots)); }
        names.extend(self.modules.borrow()[self.running_module(&stack.frames)].bound().map(|(name, _)| name.text()));
        let mut names: Vec<_> = names.into_iter().map(str::to_owned).collect();
        names.sort_unstable();
        names
    }

    /// Adds a module with no names, and gives its number.
    pub(super) fn new_module(&self) -> usize {
        let mut modules = self.modules.borrow_mut();
        modules.push(Module::new());
        modules.len() - 1
    }

    /// Each name in module `module`, with its binding, sorted by the names' text.
    pub(super) fn module_names(&self, module: usize) -> Vec<(Name, Binding)> {
        let mut names: Vec<_> = self.modules.borrow()[module].bound().map(|(name, value)| (name, value.clone())).collect();
        names.sort_by_cached_key(|(name, _)| name.text());
        names
    }

    /// Runs `f` with no current frame, at the top level of module `module`, or of the running top-level module where `module` is `None`. Then puts back the frame and module that were running.
    pub(super) fn at_top_level<T>(&self, module: Option<usize>, f: impl FnOnce() -> T) -> T {
        let (current, running) = (self.current.replace(None), self.module.get());
        if let Some(module) = module { self.module.set(module); }
        let result = f();
        self.current.set(current);
        self.module.set(running);
        result
    }

    /// The number of frames, which is the index of the next frame that a call or scope pushes.
    #[inline]
    pub(super) fn depth(&self) -> usize { self.stack.borrow().frames.len() }

    /// Pushes `frame` and makes it current. Its first slots hold `implicit`, the bindings of the six names that a call binds itself, in slot order (`Name::implicit`), and its locals' slots are empty. Fails at the depth limit, before pushing.
    #[inline]
    pub(super) fn enter(&self, mut frame: Frame, mut implicit: [Option<Binding>; IMPLICIT], span: &Span) -> Result<(), Error> {
        let mut stack = self.stack.borrow_mut();
        if stack.frames.len() == MAX_FRAME_DEPTH { return Err(Error::new(Kind::Limit, format!("lexical frame depth exceeds {MAX_FRAME_DEPTH}")).at(span)); }
        frame.base = stack.slots.len();
        stack.slots.resize_with(frame.slots().end, || None);
        stack.slots[frame.base..frame.base + IMPLICIT].swap_with_slice(&mut implicit);
        self.current.set(Some(stack.frames.len()));
        stack.frames.push(frame);
        Ok(())
    }

    /// Drops the frames from `depth` on, with their slots, as a tail call does before it pushes the frame of the call it makes.
    #[inline]
    pub(super) fn truncate(&self, depth: usize) {
        let mut stack = self.stack.borrow_mut();
        if let Some(base) = stack.frames.get(depth).map(|frame| frame.base) { stack.slots.truncate(base); }
        stack.frames.truncate(depth);
    }

    /// Drops the frames from `depth` on, and makes `current` the current frame.
    #[inline]
    pub(super) fn leave(&self, depth: usize, current: Option<usize>) {
        self.truncate(depth);
        self.current.set(current);
    }

    /// A copy of frame `i`'s bindings, for a guard case to read at its cells and then `swap` in.
    pub(super) fn frame_bindings(&self, i: usize) -> FrameBindings {
        let stack = self.stack.borrow();
        let frame = &stack.frames[i];
        FrameBindings { slots: stack.slots[frame.slots()].to_vec(), unslotted: frame.unslotted.clone() }
    }

    /// Puts each frame's bindings in, and gives back the bindings they replace.
    pub(super) fn swap(&self, bindings: Vec<(usize, FrameBindings)>) -> Vec<(usize, FrameBindings)> {
        let mut stack = self.stack.borrow_mut();
        let Stack { frames, slots } = &mut *stack;
        bindings.into_iter().map(|(i, mut copy)| {
            let frame = &mut frames[i];
            slots[frame.slots()].swap_with_slice(&mut copy.slots);
            std::mem::swap(&mut frame.unslotted, &mut copy.unslotted);
            (i, copy)
        }).collect()
    }
}
