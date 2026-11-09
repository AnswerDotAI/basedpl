mod agreement;
mod array;
mod batch;
mod cancel;
mod canvas;
mod csv;
mod data;
mod date;
mod derivative;
mod display;
mod distribution;
mod element;
mod error;
mod eval;
mod events;
mod execution;
mod explain;
#[cfg(not(web))]
mod files;
mod fusion;
#[cfg_attr(web, path = "host_web.rs")]
mod host;
mod image;
pub mod inspection;
#[cfg(not(web))]
#[doc(hidden)]
pub use basedpl_jit as jit;
pub mod js;
mod json;
pub mod keyed;
mod name;
mod number;
mod number_theory;
mod optimize;
mod pervasive;
mod plan;
mod plot;
mod polynomial;
mod primitive;
mod program;
pub mod protocol;
mod regex;
mod search;
mod selection;
pub mod settings;
pub mod symbols;
mod syntax;
pub mod system;
mod transform;
mod xml;

/// What `basedpl-testkit`'s reference checks and speed measurements take from the interpreter's internals. Not part of BPL's API.
#[doc(hidden)]
pub mod hooks {
    pub use crate::events::event_routes;
    pub use crate::explain::{Events, explained};
    pub use crate::number::Numeric;
    pub use crate::program::{blocks::compiled_runs, forget_plans};
    /// Seeds `session`'s random generator with `seed`, as each reference case's fresh session starts.
    pub fn seed_generator(session: &crate::Session, seed: u64) {
        *session.generator.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = crate::distribution::Stream::seeded(seed);
    }
}

pub use array::{Layout, Prototype, Shape, Value};
pub use cancel::{InterruptHandle, Poll};
pub(crate) use error::{At, Described};
pub use error::{Error, ErrorKind, Source, Span};
pub(crate) use eval::session;
pub use eval::{Evaluation, Function, Operand, Operator, Session, Turn};
pub use execution::{EvalOptions, Input, MimeBundle, MimeData, Output, OutputKind, OutputSink};
#[cfg(web)]
pub use host::configure_browser;
pub use inspection::Inspection;
pub use keyed::Keys;
pub use number::Number;
pub use rustymath::buffer;
pub use syntax::{ParseStatus, Parsed, SyntaxNode, highlight, in_code, number_marks, parse, subscripts, superscripts, syntax_tree};
