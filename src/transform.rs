//! Nested Derivative/Batched levels. Dispatch invokes rules; Value::read owns observation.

use crate::{array::Carried, error::Result, program::{Origin, Outputs}, session, Value};
use std::{cell::{Cell, RefCell}, sync::Arc};

#[derive(Default)]
pub(crate) struct Levels { stack: RefCell<Vec<Level>>, last: Cell<usize> }

#[derive(Clone)]
pub(crate) struct Level { pub(crate) id: usize, pub(crate) kind: LevelKind, depth: usize }

#[derive(Clone)]
pub(crate) enum LevelKind {
    Derivative,
    Batched { frame: crate::array::Layout },
}

impl Levels {
    pub(crate) fn begin(&self, kind: LevelKind) -> Level {
        self.last.set(self.last.get() + 1);
        let level = Level { id: self.last.get(), kind, depth: session().frame_depth() };
        self.stack.borrow_mut().push(level.clone());
        level
    }

    pub(crate) fn end(&self, id: usize) {
        let mut stack = self.stack.borrow_mut();
        if !stack.iter().any(|level| level.id == id) { return; }
        assert_eq!(stack.last().map(|level| level.id), Some(id), "levels end in stack order");
        stack.pop();
    }
    pub(crate) fn clear(&self) { self.stack.borrow_mut().clear(); }
    pub(crate) fn writing(&self, place: crate::eval::Place) -> Result<()> {
        for level in self.stack.borrow().iter().rev() {
            if place.frame().is_some_and(|frame| frame >= level.depth) { continue; }
            return Err(match &level.kind {
                LevelKind::Batched { .. } => crate::Error::batch_stop(level.id),
                LevelKind::Derivative => crate::Error::domain("a differentiated function cannot write outside its call"),
            });
        }
        Ok(())
    }

    pub(crate) fn find(&self, id: usize) -> Option<Level> { self.stack.borrow().iter().rev().find(|level| level.id == id).cloned() }

    pub(crate) fn innermost_batched(&self) -> Option<Level> {
        self.stack.borrow().iter().rev().find(|level| matches!(&level.kind, LevelKind::Batched { .. })).cloned()
    }
}

impl Level {
    pub(crate) fn begin(kind: LevelKind) -> Self { session().levels.begin(kind) }
    pub(crate) fn end(self) { session().levels.end(self.id); }
    pub(crate) fn find(id: usize) -> Option<Self> { session().levels.find(id) }
    pub(crate) fn innermost_batched() -> Option<Self> { session().levels.innermost_batched() }

    pub(crate) fn rule(&self, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
        match &self.kind {
            LevelKind::Derivative => crate::derivative::rule(self, origin),
            LevelKind::Batched { .. } => crate::batch::rule(self, origin),
        }
    }
}

pub(crate) struct Scope { pub(crate) level: Level }

impl Scope {
    pub(crate) fn new(kind: LevelKind) -> Self { Self { level: Level::begin(kind) } }
}

impl Drop for Scope {
    fn drop(&mut self) { session().levels.end(self.level.id); }
}

pub(crate) fn carried(level: usize, carried: Carried) -> Value { Value::Carried { level, carried: Arc::new(carried) } }

pub(crate) fn at_level(value: &Value, id: usize) -> Option<&Carried> {
    match value {
        Value::Carried { level, carried } if *level == id => Some(carried),
        _ => None,
    }
}
