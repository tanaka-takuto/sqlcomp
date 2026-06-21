use std::path::{Path, PathBuf};

use sqlay_core as core;

use crate::CompilationPlanner;

/// Default application-owned compilation planner.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompilationPlanner;

impl CompilationPlanner for DefaultCompilationPlanner {
    fn plan(&self, config: &core::ProjectConfig) -> core::DiagnosticResult<core::CompilationPlan> {
        let config_dir = config.config_dir().to_path_buf();

        Ok(core::CompilationPlan::new(
            config_dir.clone(),
            resolve_paths(&config_dir, config.source().include()),
            resolve_paths(&config_dir, config.source().exclude()),
            resolve_path(&config_dir, config.output().dir()),
            config.database().clone(),
            config.target().clone(),
        ))
    }
}

fn resolve_paths(config_dir: &Path, paths: &[String]) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| resolve_path(config_dir, path))
        .collect()
}

fn resolve_path(config_dir: &Path, path: impl AsRef<Path>) -> PathBuf {
    config_dir.join(path)
}
