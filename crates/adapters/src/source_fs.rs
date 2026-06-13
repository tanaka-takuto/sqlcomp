//! Filesystem source intake adapter.

use sqlcomp_app::SourceReader;
use sqlcomp_core as core;

/// Dummy filesystem-backed source reader.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemSourceReader;

impl SourceReader for FileSystemSourceReader {
    fn read(&self, _plan: &core::CompilationPlan) -> Vec<core::RawQuery> {
        Vec::new()
    }
}
