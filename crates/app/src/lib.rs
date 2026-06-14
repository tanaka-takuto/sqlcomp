//! Application use cases and ports.
//!
//! This crate depends only on `sqlcomp-core`. Adapter crates implement these
//! ports; `sqlcomp-app` must not depend on concrete adapters.

use sqlcomp_core as core;

use std::path::{Path, PathBuf};

/// Port for loading project configuration.
pub trait ConfigLoader {
    /// Load and validate project configuration.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when configuration cannot be found, parsed, or
    /// validated.
    fn load(&self) -> core::DiagnosticResult<core::ProjectConfig>;
}

/// Application service for constructing compilation plans.
pub trait CompilationPlanner {
    /// Convert project configuration into a resolved compilation plan.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when source, output, database, or target settings
    /// cannot be resolved into an executable plan.
    fn plan(&self, config: &core::ProjectConfig) -> core::DiagnosticResult<core::CompilationPlan>;
}

/// Port for reading SQL source files.
pub trait SourceReader {
    /// Read source files described by the compilation plan.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when source files cannot be discovered, read, or
    /// converted into raw query blocks.
    fn read(&self, plan: &core::CompilationPlan) -> core::DiagnosticResult<Vec<core::RawQuery>>;
}

/// Port for dialect-specific SQL analysis.
pub trait DialectAnalyzer {
    /// Analyze one raw query.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when SQL is invalid for the configured dialect or
    /// outside the supported MVP statement shape.
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery>;
}

/// Port for database-backed metadata lookup.
pub trait MetadataProvider {
    /// Describe database metadata for one analyzed query.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when metadata lookup cannot connect to the database
    /// or describe the analyzed query.
    fn describe(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata>;
}

/// Application service for compiling analyzed queries into core IR.
pub trait QueryCompiler {
    /// Compile one analyzed query into language-neutral IR.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when analyzed query facts and database metadata cannot
    /// be converted into the core IR.
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery>;
}

/// Port for target-language generation.
pub trait TargetGenerator {
    /// Generate target files from compiled queries.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when target-language files cannot be generated from
    /// core IR.
    fn generate(
        &self,
        queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles>;
}

/// Port for writing generated files.
pub trait GeneratedFileWriter {
    /// Persist generated files.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when generated files cannot be written.
    fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()>;
}

/// Dummy port bundle showing dependencies required by compile-like use cases.
pub trait CompileUseCasePorts {
    /// Configuration loader implementation.
    type ConfigLoader: ConfigLoader;

    /// Compilation planner implementation.
    type CompilationPlanner: CompilationPlanner;

    /// Source reader implementation.
    type SourceReader: SourceReader;

    /// Dialect analyzer implementation.
    type DialectAnalyzer: DialectAnalyzer;

    /// Metadata provider implementation.
    type MetadataProvider: MetadataProvider;

    /// Query compiler implementation.
    type QueryCompiler: QueryCompiler;

    /// Target generator implementation.
    type TargetGenerator: TargetGenerator;

    /// Generated file writer implementation.
    type GeneratedFileWriter: GeneratedFileWriter;
}

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

/// Default application-owned query compiler.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultQueryCompiler;

impl QueryCompiler for DefaultQueryCompiler {
    fn compile(
        &self,
        _query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
        _metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery> {
        Ok(core::CompiledQuery)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{CompilationPlanner, DefaultCompilationPlanner};
    use sqlcomp_core as core;

    #[test]
    fn planner_resolves_config_paths_from_config_directory() {
        let config_dir = PathBuf::from("/tmp/sqlcomp-project/packages/api");
        let config = project_config(config_dir.clone());

        let plan = DefaultCompilationPlanner
            .plan(&config)
            .expect("valid config should produce a plan");

        assert_eq!(plan.config_dir(), config_dir);
        assert_eq!(plan.source_include(), [config_dir.join("sql/**/*.sql")]);
        assert_eq!(
            plan.source_exclude(),
            [config_dir.join("sql/private/**/*.sql")]
        );
        assert_eq!(plan.output_dir(), config_dir.join("src/generated/sqlcomp"));
        assert_eq!(plan.database(), config.database());
        assert_eq!(plan.target(), config.target());
    }

    #[test]
    fn source_relative_path_uses_config_directory() {
        let config_dir = PathBuf::from("/tmp/sqlcomp-project");
        let config = project_config(config_dir.clone());
        let plan = DefaultCompilationPlanner
            .plan(&config)
            .expect("valid config should produce a plan");

        let relative_path = plan
            .source_relative_path(config_dir.join("packages/api/sql/users/list.sql"))
            .expect("source path should be inside config dir");

        assert_eq!(relative_path, Path::new("packages/api/sql/users/list.sql"));
    }

    #[test]
    fn source_relative_path_rejects_paths_outside_config_directory() {
        let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
        let plan = DefaultCompilationPlanner
            .plan(&config)
            .expect("valid config should produce a plan");

        assert_eq!(
            plan.source_relative_path("/tmp/other-project/sql/users.sql"),
            None
        );
    }

    fn project_config(config_dir: PathBuf) -> core::ProjectConfig {
        core::ProjectConfig::new(
            config_dir,
            core::SourceConfig::new(
                vec!["sql/**/*.sql".to_owned()],
                vec!["sql/private/**/*.sql".to_owned()],
            ),
            core::OutputConfig::new("src/generated/sqlcomp".to_owned()),
            core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
            core::TargetConfig::new(core::TargetLanguage::TypeScript),
        )
    }
}
