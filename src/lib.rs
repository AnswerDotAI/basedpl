mod agreement;
mod array;
mod batch;
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
mod execution;
mod explain;
#[cfg(not(web))]
mod files;
mod fusion;
#[cfg_attr(web, path = "host_web.rs")]
mod host;
mod image;
pub mod inspection;
pub mod js;
mod json;
pub mod keyed;
mod linear;
mod name;
mod number;
mod number_theory;
mod optimize;
mod pervasive;
mod plot;
mod polynomial;
mod primitive;
pub mod protocol;
#[doc(hidden)]
pub mod reference;
mod regex;
mod search;
mod selection;
pub mod settings;
pub mod symbols;
mod syntax;
pub mod system;
#[doc(hidden)]
pub mod timing;
mod transform;
mod xml;

pub use array::{Prototype, Shape, Value};
pub use rustymath::buffer;
pub(crate) use error::{At, Described};
pub use error::{Error, ErrorKind, Source, Span};
pub use eval::{Evaluation, Function, Operand, Operator, Session, Turn};
pub(crate) use eval::session;
pub use execution::{EvalOptions, Input, InterruptHandle, MimeBundle, MimeData, Output, OutputKind, OutputSink, Poll};
#[cfg(web)]
pub use host::configure_browser;
pub use inspection::Inspection;
pub use keyed::Keys;
pub use number::Number;
pub use syntax::{highlight, in_code, parse, subscripts, superscripts, syntax_tree, ParseStatus, Parsed, SyntaxNode};
