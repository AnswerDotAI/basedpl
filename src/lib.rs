mod agreement;
mod array;
pub mod cli;
mod display;
mod editor;
mod error;
mod eval;
mod execution;
mod number;
mod primitive;
mod protocol;
#[doc(hidden)]
pub mod reference;
mod selection;
mod syntax;
mod worker;

pub use array::{Array, Element};
pub use error::{Error, ErrorKind, Source, Span};
pub use eval::{Evaluation, Session};
pub use execution::{EvalOptions, InterruptHandle};
pub use number::Number;
pub use syntax::{parse, ParseStatus, Parsed};

#[cfg(feature = "python")]
mod python;
