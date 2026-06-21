use std::path::{Component, Path, PathBuf};

use crate::{DatabaseConfig, TargetConfig};

/// Resolved compilation work order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationPlan {
    config_dir: PathBuf,
    source_include: Vec<PathBuf>,
    source_exclude: Vec<PathBuf>,
    output_dir: PathBuf,
    database: DatabaseConfig,
    target: TargetConfig,
}

impl CompilationPlan {
    /// Build a resolved compilation plan.
    #[must_use]
    pub const fn new(
        config_dir: PathBuf,
        source_include: Vec<PathBuf>,
        source_exclude: Vec<PathBuf>,
        output_dir: PathBuf,
        database: DatabaseConfig,
        target: TargetConfig,
    ) -> Self {
        Self {
            config_dir,
            source_include,
            source_exclude,
            output_dir,
            database,
            target,
        }
    }

    /// Directory containing `sqlay.config.json`.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Include glob patterns resolved relative to the configuration directory.
    #[must_use]
    pub fn source_include(&self) -> &[PathBuf] {
        &self.source_include
    }

    /// Exclude glob patterns resolved relative to the configuration directory.
    #[must_use]
    pub fn source_exclude(&self) -> &[PathBuf] {
        &self.source_exclude
    }

    /// Generated output directory resolved relative to the configuration directory.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Database metadata settings for this compile run.
    #[must_use]
    pub const fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    /// Target-language settings for this compile run.
    #[must_use]
    pub const fn target(&self) -> &TargetConfig {
        &self.target
    }

    /// Return a source path relative to the configuration directory.
    #[must_use]
    pub fn source_relative_path(&self, source_path: impl AsRef<Path>) -> Option<PathBuf> {
        let relative_path = source_path
            .as_ref()
            .strip_prefix(&self.config_dir)
            .ok()?
            .to_path_buf();

        is_safe_relative_path(&relative_path).then_some(relative_path)
    }
}

fn is_safe_relative_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::CurDir | Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::{CompilationPlan, DatabaseConfig, DatabaseDialect, TargetConfig, TargetLanguage};

    #[test]
    fn source_relative_path_returns_config_relative_path() {
        let config_dir = PathBuf::from("/tmp/sqlay-project");
        let plan = compilation_plan(config_dir.clone());

        let relative_path = plan
            .source_relative_path(config_dir.join("sql/users/list.sql"))
            .expect("source path should be inside config dir");

        assert_eq!(relative_path, Path::new("sql/users/list.sql"));
    }

    #[test]
    fn source_relative_path_rejects_parent_dir_after_config_prefix() {
        let config_dir = PathBuf::from("/tmp/sqlay-project");
        let plan = compilation_plan(config_dir.clone());

        assert_eq!(
            plan.source_relative_path(config_dir.join("../shared/users.sql")),
            None
        );
    }

    fn compilation_plan(config_dir: PathBuf) -> CompilationPlan {
        CompilationPlan::new(
            config_dir,
            vec![PathBuf::from("sql/**/*.sql")],
            Vec::new(),
            PathBuf::from("src/generated/sqlay"),
            DatabaseConfig::new(DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
            TargetConfig::new(TargetLanguage::TypeScript),
        )
    }
}
