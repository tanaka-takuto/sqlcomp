//! Filesystem source intake adapter.

mod diagnostics;
mod discovery;
mod inline_markers;
mod metadata;
mod reader;
mod scanner;
mod source_units;

#[cfg(test)]
mod tests;

pub use metadata::parse_sqlay_query_metadata;
pub use reader::FileSystemSourceReader;
pub use scanner::{SqlayBlock, SqlayBlockScan, scan_sqlay_blocks};
pub use source_units::split_sqlay_query_blocks;
