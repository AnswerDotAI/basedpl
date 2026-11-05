//! Nested Derivative/Batched levels. Dispatch invokes rules; Value::read owns observation.
//! A larger id is a newer transform. Rules dispatch calls on their parts, exposing older levels to those calls.

use crate::{Value, array::Carried, error::Result, primitive::{CallView, Operation, ReadOp, ViewAxis}, program::{Origin, Outputs}, session};
use std::{cell::{Cell, RefCell}, sync::Arc};

#[derive(Default)]
pub(crate) struct Levels { stack: RefCell<Vec<Running>>, last: Cell<usize>, batch_rule: Cell<Option<usize>> }

#[derive(Clone)]
pub(crate) struct Level { pub(crate) id: usize, pub(crate) kind: LevelKind }
struct Running { level: Level, depth: usize, handling: bool, writes: Vec<crate::eval::Place> }

#[derive(Clone)]
pub(crate) enum LevelKind { Derivative, Batched { frame: Value } }

impl Levels {
    pub(crate) fn begin(&self, kind: LevelKind) -> Level {
        self.last.set(self.last.get() + 1);
        let level = Level { id: self.last.get(), kind };
        self.stack.borrow_mut().push(Running { level: level.clone(), depth: session().frame_depth(), handling: false, writes: Vec::new() });
        level
    }

    pub(crate) fn end(&self, id: usize) -> Vec<crate::eval::Place> {
        let mut stack = self.stack.borrow_mut();
        if !stack.iter().any(|running| running.level.id == id) { return Vec::new(); }
        assert_eq!(stack.last().map(|running| running.level.id), Some(id), "levels end in stack order");
        stack.pop().expect("a running transform level").writes
    }
    pub(crate) fn clear(&self) {
        self.stack.borrow_mut().clear();
        self.batch_rule.set(None);
    }
    pub(crate) fn writing(&self, place: crate::eval::Place) -> Result<()> {
        for running in self.stack.borrow_mut().iter_mut().rev() {
            if place.frame().is_some_and(|frame| frame >= running.depth) { continue; }
            match &running.level.kind {
                LevelKind::Batched { .. } => return Err(crate::Error::batch_stop(running.level.id)),
                LevelKind::Derivative => if !running.writes.contains(&place) { running.writes.push(place); },
            }
        }
        Ok(())
    }

    pub(crate) fn find(&self, id: usize) -> Option<Level> { self.stack.borrow().iter().rev().find(|running| running.level.id == id).map(|running| running.level.clone()) }
    fn set_handling(&self, id: usize, handling: bool) -> bool {
        self.stack.borrow_mut().iter_mut().find(|running| running.level.id == id).is_some_and(|running| std::mem::replace(&mut running.handling, handling))
    }

    /// Exclude the batch whose rule is running from implicit resource dispatch. Explicit carried operands still select their own level.
    pub(crate) fn innermost_batched(&self) -> Option<Level> {
        self.stack.borrow().iter().rev().find(|running| !running.handling && matches!(&running.level.kind, LevelKind::Batched { .. })).map(|running| running.level.clone())
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

impl Drop for Scope { fn drop(&mut self) { session().end_transform(self.level.id); } }

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
        frames.push((offset..(offset + rank).min(value.rank())).map(|axis| ViewAxis { input, axis: Some(axis) }).collect());
        arguments.push(value.clone());
    }
    Operation::Read(ReadOp::Frame { frames: frames.into() }).call(&arguments)
}
pub(crate) fn carried(level: usize, carried: Carried) -> Value { Value::Carried { level, carried: Arc::new(carried) } }

pub(crate) fn at_level(value: &Value, id: usize) -> Option<&Carried> {
    match value { Value::Carried { level, carried } if *level == id => Some(carried), _ => None }
}
