//! TypeScript target generation adapter.

use sqlcomp_app::TargetGenerator;
use sqlcomp_core as core;

/// Dummy TypeScript target generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptTargetGenerator;

impl TargetGenerator for TypeScriptTargetGenerator {
    fn generate(&self, _queries: &[core::CompiledQuery]) -> core::GeneratedFiles {
        core::GeneratedFiles
    }
}
