//! Nested Derivative/Batched levels. Dispatch invokes rules; Value::read owns observation.

use crate::{Value, array::Carried, error::Result, primitive::{CallView, Operation, ReadOp, ViewAxis}, program::{Origin, Outputs}, session};
use std::{cell::{Cell, RefCell}, sync::Arc};

#[derive(Default)]
pub(crate) struct Levels { stack: RefCell<Vec<Level>>, last: Cell<usize>, batch_rule: Cell<Option<usize>> }

#[derive(Clone)]
pub(crate) struct Level { pub(crate) id: usize, pub(crate) kind: LevelKind, depth: usize, handling: bool }

#[derive(Clone)]
pub(crate) enum LevelKind { Derivative, Batched { frame: Value } }

impl Levels {
    pub(crate) fn begin(&self, kind: LevelKind) -> Level {
        self.last.set(self.last.get() + 1);
        let level = Level { id: self.last.get(), kind, depth: session().frame_depth(), handling: false };
        self.stack.borrow_mut().push(level.clone());
        level
    }

    pub(crate) fn end(&self, id: usize) {
        let mut stack = self.stack.borrow_mut();
        if !stack.iter().any(|level| level.id == id) { return; }
        assert_eq!(stack.last().map(|level| level.id), Some(id), "levels end in stack order");
        stack.pop();
    }
    pub(crate) fn clear(&self) {
        self.stack.borrow_mut().clear();
        self.batch_rule.set(None);
    }
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
    fn set_handling(&self, id: usize, handling: bool) -> bool {
        self.stack.borrow_mut().iter_mut().find(|level| level.id == id).is_some_and(|level| std::mem::replace(&mut level.handling, handling))
    }

    pub(crate) fn innermost_batched(&self) -> Option<Level> {
        self.stack.borrow().iter().rev().find(|level| !level.handling && matches!(&level.kind, LevelKind::Batched { .. })).cloned()
    }
    pub(crate) fn current_batch_rule(&self) -> Option<Level> { self.batch_rule.get().and_then(|id| self.find(id)) }
}

impl Level {
    pub(crate) fn begin(kind: LevelKind) -> Self { session().levels.begin(kind) }
    pub(crate) fn find(id: usize) -> Option<Self> { session().levels.find(id) }
    pub(crate) fn innermost_batched() -> Option<Self> { session().levels.innermost_batched() }
    pub(crate) fn current_batch_rule() -> Option<Self> { session().levels.current_batch_rule() }

    pub(crate) fn frame_reference(&self) -> Result<Value> {
        let LevelKind::Batched { frame } = &self.kind else { return Err(crate::Error::internal()); };
        Ok(frame.clone())
    }

    pub(crate) fn rule(&self, origin: Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>> {
        let previous = session().levels.set_handling(self.id, true);
        let result = match &self.kind {
            LevelKind::Derivative => crate::derivative::rule(self, origin, call),
            LevelKind::Batched { .. } => {
                let previous = session().levels.batch_rule.replace(Some(self.id));
                let result = crate::batch::rule(self, origin, call);
                session().levels.batch_rule.set(previous);
                result
            }
        };
        session().levels.set_handling(self.id, previous);
        result
    }
}

pub(crate) struct Scope { pub(crate) level: Level }

impl Scope { pub(crate) fn new(kind: LevelKind) -> Self { Self { level: Level::begin(kind) } } }

impl Drop for Scope { fn drop(&mut self) { session().levels.end(self.level.id); } }

/// A frame from actual runtime sources; all references are description-only inputs.
pub(crate) fn frame_reference(references: &[Value], axes: &[ViewAxis]) -> Result<Value> {
    let arguments = std::iter::once(Value::constant(Value::integer(0))).chain(references.iter().cloned()).collect::<Vec<_>>();
    Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: Box::new([]) }).call(&arguments)
}

pub(crate) fn agreed_frame(references: &[Value], rank: usize) -> Result<Value> {
    let mut arguments = vec![Value::constant(Value::Number(crate::Number::from_bool(false)))];
    let mut frames = Vec::with_capacity(references.len());
    for reference in references {
        let (mut value, mut offset) = (reference, 0);
        while let Value::Carried { carried, .. } = value {
            value = match &**carried {
                Carried::Dual { primal, .. } => primal,
                Carried::Batch { whole, cell } => {
                    offset += whole.rank() - cell.shape().len();
                    whole
                }
            };
        }
        let input = arguments.len() as u32;
        frames.push((offset..offset + rank).map(|axis| ViewAxis { input, axis: Some(axis) }).collect());
        arguments.push(value.clone());
    }
    Operation::Read(ReadOp::Frame { frames: frames.into() }).call(&arguments)
}
pub(crate) fn carried(level: usize, carried: Carried) -> Value { Value::Carried { level, carried: Arc::new(carried) } }

pub(crate) fn at_level(value: &Value, id: usize) -> Option<&Carried> {
    match value { Value::Carried { level, carried } if *level == id => Some(carried), _ => None }
}
