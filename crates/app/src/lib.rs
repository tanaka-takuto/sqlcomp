//! Application use cases and ports.
//!
//! This crate depends only on `sqlcomp-core`. Adapter crates implement these
//! ports; `sqlcomp-app` must not depend on concrete adapters.

use sqlcomp_core as core;

use std::path::{Path, PathBuf};

/// Standard project configuration file name.
pub const CONFIG_FILE_NAME: &str = "sqlcomp.config.json";

/// Starter configuration written by `sqlcomp init`.
pub const STARTER_CONFIG_TEMPLATE: &str = r#"{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": []
  },
  "output": {
    "dir": "src/generated/sqlcomp"
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
"#;

/// Port for creating a starter project configuration file.
pub trait ConfigTemplateWriter {
    /// Write starter configuration content to a new file.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the target file already exists or cannot be
    /// written.
    fn write_new(&self, path: &Path, contents: &str) -> core::DiagnosticResult<()>;
}

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
        plan: &core::CompilationPlan,
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

/// Application service for initializing a sqlcomp project config.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultProjectInitializer;

impl DefaultProjectInitializer {
    /// Create the starter config in `current_dir`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the config file already exists or cannot be
    /// written.
    pub fn init(
        current_dir: &Path,
        writer: &impl ConfigTemplateWriter,
    ) -> core::DiagnosticResult<PathBuf> {
        let config_path = current_dir.join(CONFIG_FILE_NAME);
        writer.write_new(&config_path, STARTER_CONFIG_TEMPLATE)?;

        Ok(config_path)
    }
}

/// Application service for compile-like CLI commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompileUseCase;

impl DefaultCompileUseCase {
    /// Run the `check` command skeleton.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning fails or when the downstream compile
    /// pipeline is not implemented yet.
    pub fn check(
        config: &core::ProjectConfig,
        planner: &impl CompilationPlanner,
    ) -> core::DiagnosticResult<()> {
        let plan = planner.plan(config)?;

        Err(compile_pipeline_pending("check", &plan))
    }

    /// Run the `compile` command skeleton.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning fails or when the downstream compile
    /// pipeline is not implemented yet.
    pub fn compile(
        config: &core::ProjectConfig,
        planner: &impl CompilationPlanner,
        _clean: bool,
    ) -> core::DiagnosticResult<()> {
        let plan = planner.plan(config)?;

        Err(compile_pipeline_pending("compile", &plan))
    }
}

fn compile_pipeline_pending(
    command: &str,
    _plan: &core::CompilationPlan,
) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(format!(
        "command `{command}` loaded configuration, but the compile pipeline is not implemented yet"
    )))
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
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery> {
        let cardinality = query
            .metadata()
            .cardinality()
            .unwrap_or_else(|| analysis.cardinality());
        let row = metadata
            .columns()
            .iter()
            .map(core::DbResultColumn::to_result_column)
            .collect();

        let mut compiled = core::CompiledQuery::new(
            core::QueryId::new(query.metadata().id().to_owned()),
            query.sql().to_owned(),
            cardinality,
            Vec::new(),
            row,
        );

        if let Some(source_path) = query.source_path() {
            compiled = compiled.with_source_path(source_path.to_path_buf());
        }

        Ok(compiled)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        CompilationPlanner, DefaultCompilationPlanner, DefaultCompileUseCase, DefaultQueryCompiler,
        QueryCompiler,
    };
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

    #[test]
    fn check_command_reaches_compile_pipeline_skeleton_after_planning() {
        let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
        let report = DefaultCompileUseCase::check(&config, &DefaultCompilationPlanner)
            .expect_err("pipeline skeleton should report that deeper compile is pending");

        assert_eq!(
            diagnostic_messages(&report),
            "command `check` loaded configuration, but the compile pipeline is not implemented yet"
        );
    }

    #[test]
    fn compile_command_reaches_compile_pipeline_skeleton_after_planning() {
        let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
        let report = DefaultCompileUseCase::compile(&config, &DefaultCompilationPlanner, false)
            .expect_err("pipeline skeleton should report that deeper compile is pending");

        assert_eq!(
            diagnostic_messages(&report),
            "command `compile` loaded configuration, but the compile pipeline is not implemented yet"
        );
    }

    #[test]
    fn query_compiler_builds_core_ir_with_empty_mvp_input_and_result_columns() {
        let query = core::RawQuery::new(
            core::QueryMetadata::new("listUsers".to_owned(), None),
            "SELECT id, name FROM users;".to_owned(),
        )
        .with_source_path("sql/users.sql");
        let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
        let metadata = core::DbQueryMetadata::new(vec![
            core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
            core::DbResultColumn::new("name".to_owned(), core::CoreType::String, Some(true)),
        ]);

        let compiled = DefaultQueryCompiler
            .compile(&query, &analysis, &metadata)
            .expect("query should compile into core IR");

        assert_eq!(compiled.id().as_str(), "listUsers");
        assert_eq!(compiled.source_path(), Some(Path::new("sql/users.sql")));
        assert_eq!(compiled.sql(), "SELECT id, name FROM users;");
        assert_eq!(compiled.cardinality(), core::Cardinality::Many);
        assert!(compiled.input().is_empty());
        assert_eq!(compiled.row().len(), 2);
        assert_eq!(compiled.row()[0].name(), "id");
        assert_eq!(compiled.row()[0].ty(), core::CoreType::Int64);
        assert!(!compiled.row()[0].is_nullable());
        assert_eq!(compiled.row()[1].name(), "name");
        assert_eq!(compiled.row()[1].ty(), core::CoreType::String);
        assert!(compiled.row()[1].is_nullable());
    }

    #[test]
    fn query_compiler_uses_inferred_cardinality_when_metadata_has_no_override() {
        let compiled = compile_query(None, core::Cardinality::Many);

        assert_eq!(compiled.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn query_compiler_uses_explicit_one_cardinality_over_inference() {
        let compiled = compile_query(Some(core::Cardinality::One), core::Cardinality::Many);

        assert_eq!(compiled.cardinality(), core::Cardinality::One);
    }

    #[test]
    fn query_compiler_uses_explicit_many_cardinality_over_inference() {
        let compiled = compile_query(Some(core::Cardinality::Many), core::Cardinality::One);

        assert_eq!(compiled.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn query_compiler_copies_database_columns_to_result_row() {
        let query = core::RawQuery::new(
            core::QueryMetadata::new("listUsers".to_owned(), None),
            "SELECT id, nickname FROM users;".to_owned(),
        );
        let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
        let metadata = core::DbQueryMetadata::new(vec![
            core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
            core::DbResultColumn::new("nickname".to_owned(), core::CoreType::String, Some(true)),
        ]);

        let compiled = DefaultQueryCompiler
            .compile(&query, &analysis, &metadata)
            .expect("query compiler should preserve result row metadata");

        assert_eq!(
            compiled.row(),
            [
                core::ResultColumn::new("id".to_owned(), core::CoreType::Int64, false),
                core::ResultColumn::new("nickname".to_owned(), core::CoreType::String, true),
            ]
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

    fn compile_query(
        explicit_cardinality: Option<core::Cardinality>,
        inferred_cardinality: core::Cardinality,
    ) -> core::CompiledQuery {
        let query = core::RawQuery::new(
            core::QueryMetadata::new("listUsers".to_owned(), explicit_cardinality),
            "SELECT id FROM users;".to_owned(),
        );
        let analysis = core::AnalyzedQuery::new(inferred_cardinality);

        DefaultQueryCompiler
            .compile(&query, &analysis, &core::DbQueryMetadata::new(Vec::new()))
            .expect("query compiler should resolve cardinality")
    }

    fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
        report
            .diagnostics()
            .iter()
            .map(core::Diagnostic::message)
            .collect::<Vec<_>>()
            .join("\n")
    }
}
