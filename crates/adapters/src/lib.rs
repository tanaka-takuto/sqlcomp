//! Outer adapter implementations.
//!
//! This crate contains infrastructure adapters behind `sqlcomp-app` ports. The
//! crate may depend on `sqlcomp-app` and `sqlcomp-core`, but it should not depend
//! on `sqlcomp-cli` or on sibling outer modules through separate crates.

pub mod config_jsonc;
pub mod dialect_mysql;
pub mod metadata_mysql_sqlx;
pub mod output_fs;
pub mod source_fs;
pub mod target;
