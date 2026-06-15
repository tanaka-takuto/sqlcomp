//! CLI driver boundary.
//!
//! The CLI is the composition root. It wires application ports to concrete
//! adapters and is the only crate that should depend on all adapter crates.

mod args;
mod diagnostics;
mod help;
mod output;
mod runtime;

pub use runtime::{DefaultPipeline, run};
