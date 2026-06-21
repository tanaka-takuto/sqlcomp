//! Outer adapter implementations.
//!
//! This crate contains infrastructure adapters behind `sqlay-app` ports. The
//! crate may depend on `sqlay-app` and `sqlay-core`, but it should not depend
//! on `sqlay-cli` or on sibling outer modules through separate crates.

pub mod config_jsonc;
pub mod dialect_mysql;
pub mod metadata;
pub mod output_fs;
pub mod source_fs;
pub mod target;
