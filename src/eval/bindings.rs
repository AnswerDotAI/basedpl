//! Binding storage: the frames of running calls and scopes, each module's names, and where code is running. Code finds where a name is bound as a `Place`, and reads and writes bindings only through `Bindings`.

use super::{Binding, Category, MAX_FRAME_DEPTH};
use crate::{Error, Span, Value, name::Name};
use rustymath::{error::Kind, hashing::{HashMap, HashMapExt, HashSet, HashSetExt}};
use std::{cell::{Cell, RefCell}, ops::Range};

/// Where a binding is, or where an assignment puts one: a frame's slot, by the frame's index and the slot's number among the frame's slots; a name that a frame binds without a slot, such as a key of a scope's record; or a module name, by the module number and interned name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place { Slot(usize, u32), Other(usize, Name), Entry(usize, Name) }

impl Place {
    /// The frame that holds the binding, or `None` for a module's names, which outlive every frame. A binding can't hold a value that refers to a later frame.
    pub(crate) fn frame(self) -> Option<usize> { match self { Self::Slot(i, _) | Self::Other(i, _) => Some(i), Self::Entry(..) => None } }
}

/// The number of slots before a frame's locals: one for each of the six names that a call binds itself, in the order of `Name::implicit`.
const IMPLICIT: usize = 6;

/// What a frame binds without slots: names that `⍎` binds where the frame's definition has no slot for them, `$e`, a scope's assignments, and a scope's record, whose keys on its first axis name its items or major cells (`Frame::key`). A key's text never becomes a `Name`, so a record's keys never add to the process's names.
#[derive(Default)]
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
    pub(super) fn scope(parent: Option<usize>, module: usize, record: Value) -> Self {
        Self { unslotted: Some(Box::new(Unslotted { names: HashMap::new(), record: Some(record) })), ..Self::new(parent, module, &[]) }
    }

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
        match self.slot(name) {
            Some(s) => Some((Place::Slot(i, s), slots[self.base + s as usize].as_ref()?)), None => Some((Place::Other(i, name), self.other(name)?))
        }
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
        let locals = self.locals.iter().zip(&slots[self.base + IMPLICIT..]).filter(|(_, slot)| !matches!(slot, None | Some(Binding::Released)))
            .map(|(name, _)| name.text());
        let keys = |record: &'a Value| record.keys(0).into_iter().flat_map(|keys| keys.names().iter().flatten().map(|key| &**key));
        locals.chain(self.unslotted.iter().flat_map(move |u| u.names.keys().map(|name| name.text()).chain(u.record.iter().flat_map(keys))))
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

/// Every binding, and where code is running: the frames of running calls and scopes with their slots, each module's names, the current frame, and the module whose top level is running.
pub(super) struct Bindings {
    stack: RefCell<Stack>,
    /// The names of each module, by module number. Module 0 holds the session's own names, and each loaded file adds a module.
    modules: RefCell<Vec<HashMap<Name, Binding>>>,
    /// The frame that code is running in, or `None` at a module's top level.
    current: Cell<Option<usize>>,
    /// The module whose top level is running.
    module: Cell<usize>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self { stack: RefCell::default(), modules: RefCell::new(vec![HashMap::new()]), current: Cell::default(), module: Cell::default() }
    }
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
            None => Place::Entry(self.module.get(), name),
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

    /// `f` on the binding of `name` that code sees here, and its place. Lookup tries the current frame, then each frame that it sees names through, then the running module's names. Every call's frame binds `α`, to an absent argument where the call has no left argument, so lookup finds the current call's `α`. No module holds the six names that a call binds itself. In each frame, a name that has a slot is in its slot or unbound there, and an empty slot sends the lookup on to the next frame, as an unbound name does. In a scope's frame, a key of its record names its item or major cell, after any name that an assignment in the scope binds. The binding stays borrowed while `f` reads it, so `f` evaluates nothing.
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
        self.modules.borrow()[module].get(&name).map(|value| f(Place::Entry(module, name), value))
    }

    /// The binding at `place`, copied. `None` where nothing is bound there, or `place` is in a frame that has ended.
    #[inline]
    pub(super) fn get(&self, place: Place) -> Option<Binding> {
        match place {
            Place::Slot(i, s) => { let stack = self.stack.borrow(); stack.slots[stack.frames.get(i)?.base + s as usize].clone() }
            Place::Other(i, name) => {
                let stack = self.stack.borrow();
                let frame = stack.frames.get(i)?;
                frame.other(name).cloned().or_else(|| frame.key(name))
            }
            Place::Entry(m, name) => self.modules.borrow()[m].get(&name).cloned(),
        }
    }

    /// Binds `place` to `value`.
    #[inline]
    pub(super) fn insert(&self, place: Place, value: Binding) {
        match place {
            Place::Slot(i, s) => {
                let mut stack = self.stack.borrow_mut();
                let at = stack.frames[i].base + s as usize;
                stack.slots[at] = Some(value);
            }
            Place::Other(i, name) => { self.stack.borrow_mut().frames[i].unslotted.get_or_insert_default().names.insert(name, value); }
            Place::Entry(m, name) => { self.modules.borrow_mut()[m].insert(name, value); }
        }
    }

    /// Removes the binding at `place`, and gives it back. A frame slot stays empty.
    pub(super) fn remove(&self, place: Place) -> Option<Binding> {
        match place {
            Place::Slot(i, s) => {
                let mut stack = self.stack.borrow_mut();
                let at = stack.frames[i].base + s as usize;
                stack.slots[at].take()
            }
            Place::Other(i, name) => self.stack.borrow_mut().frames[i].unslotted.as_mut()?.names.remove(&name),
            Place::Entry(m, name) => self.modules.borrow_mut()[m].remove(&name),
        }
    }

    /// The binding of `name` in the current frame's slot, taken out, with its place, for a dfn's last read of a local, `ω` or `α` (`syntax::release`). `None` where that slot holds nothing.
    pub(super) fn take(&self, name: Name) -> Option<(Place, Binding)> {
        let s = { let stack = self.stack.borrow(); stack.frames[self.current.get()?].slot(name)? };
        self.take_slot(s)
    }

    /// The binding in slot `s` of the current frame, taken out, leaving `Binding::Released`. Lookup then stops at the slot, so a later read by text fails there and never finds an outer binding of the name.
    fn take_slot(&self, s: u32) -> Option<(Place, Binding)> {
        let i = self.current.get()?;
        let mut stack = self.stack.borrow_mut();
        let base = stack.frames[i].base;
        let slot = &mut stack.slots[base + s as usize];
        if matches!(slot, None | Some(Binding::Released)) { return None; }
        slot.replace(Binding::Released).map(|binding| (Place::Slot(i, s), binding))
    }

    /// The text of each name that code sees here, once each, sorted: the names of the current frame and of each frame that it sees names through, then the running module's. The six names that a call binds itself, and empty slots, aren't listed.
    pub(super) fn names(&self) -> Vec<String> {
        let stack = self.stack.borrow();
        let mut names = HashSet::new();
        for (_, frame) in self.chain(&stack.frames) { names.extend(frame.names(&stack.slots)); }
        names.extend(self.modules.borrow()[self.running_module(&stack.frames)].keys().map(|name| name.text()));
        let mut names: Vec<_> = names.into_iter().map(str::to_owned).collect();
        names.sort_unstable();
        names
    }

    /// Adds a module with no names, and gives its number.
    pub(super) fn new_module(&self) -> usize {
        let mut modules = self.modules.borrow_mut();
        modules.push(HashMap::new());
        modules.len() - 1
    }

    /// Each name in module `module`, with its binding, sorted by the names' text.
    pub(super) fn module_names(&self, module: usize) -> Vec<(Name, Binding)> {
        let mut names: Vec<_> = self.modules.borrow()[module].iter().map(|(&name, value)| (name, value.clone())).collect();
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
    // Out of line in the browser, to keep its locals out of the frame that `Session::call_defined` keeps on the stack at every level of recursion, as `Binder::reduce`'s comment explains.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
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
}
