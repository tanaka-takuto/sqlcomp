//! Filesystem generated-output adapter.

use sqlcomp_app::GeneratedFileWriter;
use sqlcomp_core as core;

/// Dummy filesystem-backed generated file writer.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemGeneratedFileWriter;

impl GeneratedFileWriter for FileSystemGeneratedFileWriter {
    fn write(&self, _files: &core::GeneratedFiles) {}
}
