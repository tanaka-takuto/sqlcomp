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

/// Port for removing stale managed generated files.
pub trait GeneratedFileCleaner {
    /// Remove generated files under `output_dir` that are managed by sqlcomp and
    /// not present in `current_files`.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when generated files cannot be inspected or removed.
    fn clean_stale(
        &self,
        output_dir: &Path,
        current_files: &core::GeneratedFiles,
    ) -> core::DiagnosticResult<()>;
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

/// Concrete port references required to run the compile pipeline.
#[derive(Clone, Copy, Debug)]
pub struct CompilePipeline<'a, P, S, D, M, Q, T, W>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer,
    M: MetadataProvider,
    Q: QueryCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter + GeneratedFileCleaner,
{
    /// Compilation planner implementation.
    pub planner: &'a P,
    /// SQL source reader implementation.
    pub source_reader: &'a S,
    /// Dialect analyzer implementation.
    pub dialect_analyzer: &'a D,
    /// Database metadata provider implementation.
    pub metadata_provider: &'a M,
    /// Core IR compiler implementation.
    pub query_compiler: &'a Q,
    /// Target-language generator implementation.
    pub target_generator: &'a T,
    /// Generated file writer and cleaner implementation.
    pub generated_file_writer: &'a W,
}

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

    /// Run the `compile` command.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, generation, or file writing fails.
    pub fn compile<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
        clean: bool,
    ) -> core::DiagnosticResult<()>
    where
        P: CompilationPlanner,
        S: SourceReader,
        D: DialectAnalyzer,
        M: MetadataProvider,
        Q: QueryCompiler,
        T: TargetGenerator,
        W: GeneratedFileWriter + GeneratedFileCleaner,
    {
        let plan = pipeline.planner.plan(config)?;

        let raw_queries = pipeline.source_reader.read(&plan)?;
        let mut compiled_queries = Vec::with_capacity(raw_queries.len());

        for query in &raw_queries {
            let analysis = pipeline.dialect_analyzer.analyze(query)?;
            let metadata = pipeline.metadata_provider.describe(query, &analysis)?;
            let compiled = pipeline
                .query_compiler
                .compile(query, &analysis, &metadata)?;
            compiled_queries.push(compiled);
        }

        let generated_files = pipeline
            .target_generator
            .generate(&plan, &compiled_queries)?;
        pipeline.generated_file_writer.write(&generated_files)?;

        if clean {
            pipeline
                .generated_file_writer
                .clean_stale(plan.output_dir(), &generated_files)?;
        }

        Ok(())
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

    /// Generated file writer and cleaner implementation.
    type GeneratedFileWriter: GeneratedFileWriter + GeneratedFileCleaner;
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
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    use super::{
        CompilationPlanner, CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase,
        DefaultQueryCompiler, DialectAnalyzer, GeneratedFileCleaner, GeneratedFileWriter,
        MetadataProvider, QueryCompiler, SourceReader, TargetGenerator,
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
    fn compile_command_writes_generated_files_from_pipeline() {
        let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
        let written = RefCell::new(None);
        let cleaned = RefCell::new(None);
        let writer = RecordingGeneratedFileWriter {
            written: &written,
            cleaned: &cleaned,
        };
        let pipeline = CompilePipeline {
            planner: &DefaultCompilationPlanner,
            source_reader: &FakeSourceReader,
            dialect_analyzer: &FakeDialectAnalyzer,
            metadata_provider: &FakeMetadataProvider,
            query_compiler: &DefaultQueryCompiler,
            target_generator: &FakeTargetGenerator,
            generated_file_writer: &writer,
        };

        DefaultCompileUseCase::compile(&config, &pipeline, false)
            .expect("compile should run the pipeline and write generated files");

        let files = written
            .into_inner()
            .expect("compile should pass generated files to the writer");
        assert_eq!(files.files().len(), 1);
        assert_eq!(
            files.files()[0].path(),
            Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/users.ts")
        );
        assert_eq!(files.files()[0].contents(), "generated listUsers\n");
        assert!(
            cleaned.into_inner().is_none(),
            "normal compile should leave stale files untouched"
        );
    }

    #[test]
    fn compile_clean_writes_generated_files_and_removes_stale_files() {
        let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
        let written = RefCell::new(None);
        let cleaned = RefCell::new(None);
        let writer = RecordingGeneratedFileWriter {
            written: &written,
            cleaned: &cleaned,
        };
        let pipeline = CompilePipeline {
            planner: &DefaultCompilationPlanner,
            source_reader: &FakeSourceReader,
            dialect_analyzer: &FakeDialectAnalyzer,
            metadata_provider: &FakeMetadataProvider,
            query_compiler: &DefaultQueryCompiler,
            target_generator: &FakeTargetGenerator,
            generated_file_writer: &writer,
        };

        DefaultCompileUseCase::compile(&config, &pipeline, true)
            .expect("compile --clean should run generation and cleanup");

        let files = written
            .into_inner()
            .expect("compile --clean should write generated files");
        let (output_dir, current_files) = cleaned
            .into_inner()
            .expect("compile --clean should clean stale generated files");
        assert_eq!(
            output_dir,
            PathBuf::from("/tmp/sqlcomp-project/src/generated/sqlcomp")
        );
        assert_eq!(current_files, files);
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

    #[test]
    fn query_compiler_maps_unknown_nullability_to_nullable_result_row() {
        let query = core::RawQuery::new(
            core::QueryMetadata::new("inspectUsers".to_owned(), None),
            "SELECT id, nickname, computed_name FROM users;".to_owned(),
        );
        let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
        let metadata = core::DbQueryMetadata::new(vec![
            core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
            core::DbResultColumn::new("nickname".to_owned(), core::CoreType::String, Some(true)),
            core::DbResultColumn::new("computed_name".to_owned(), core::CoreType::String, None),
        ]);

        let compiled = DefaultQueryCompiler
            .compile(&query, &analysis, &metadata)
            .expect("query compiler should preserve conservative nullability");

        assert_eq!(
            compiled.row(),
            [
                core::ResultColumn::new("id".to_owned(), core::CoreType::Int64, false),
                core::ResultColumn::new("nickname".to_owned(), core::CoreType::String, true),
                core::ResultColumn::new("computed_name".to_owned(), core::CoreType::String, true),
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

    #[derive(Clone, Copy, Debug)]
    struct FakeSourceReader;

    impl SourceReader for FakeSourceReader {
        fn read(
            &self,
            _plan: &core::CompilationPlan,
        ) -> core::DiagnosticResult<Vec<core::RawQuery>> {
            Ok(vec![
                core::RawQuery::new(
                    core::QueryMetadata::new("listUsers".to_owned(), None),
                    "SELECT id FROM users;".to_owned(),
                )
                .with_source_path("sql/users.sql"),
            ])
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FakeDialectAnalyzer;

    impl DialectAnalyzer for FakeDialectAnalyzer {
        fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
            assert_eq!(query.metadata().id(), "listUsers");

            Ok(core::AnalyzedQuery::new(core::Cardinality::Many))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FakeMetadataProvider;

    impl MetadataProvider for FakeMetadataProvider {
        fn describe(
            &self,
            query: &core::RawQuery,
            analysis: &core::AnalyzedQuery,
        ) -> core::DiagnosticResult<core::DbQueryMetadata> {
            assert_eq!(query.metadata().id(), "listUsers");
            assert_eq!(analysis.cardinality(), core::Cardinality::Many);

            Ok(core::DbQueryMetadata::new(vec![core::DbResultColumn::new(
                "id".to_owned(),
                core::CoreType::Int64,
                Some(false),
            )]))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FakeTargetGenerator;

    impl TargetGenerator for FakeTargetGenerator {
        fn generate(
            &self,
            plan: &core::CompilationPlan,
            queries: &[core::CompiledQuery],
        ) -> core::DiagnosticResult<core::GeneratedFiles> {
            let [query] = queries else {
                panic!("expected exactly one compiled query");
            };
            assert_eq!(query.id().as_str(), "listUsers");
            assert_eq!(query.source_path(), Some(Path::new("sql/users.sql")));
            assert_eq!(query.row()[0].ty(), core::CoreType::Int64);

            Ok(core::GeneratedFiles::new(vec![core::GeneratedFile::new(
                plan.output_dir().join("sql/users.ts"),
                format!("generated {}\n", query.id().as_str()),
            )]))
        }
    }

    #[derive(Debug)]
    struct RecordingGeneratedFileWriter<'a> {
        written: &'a RefCell<Option<core::GeneratedFiles>>,
        cleaned: &'a RefCell<Option<(PathBuf, core::GeneratedFiles)>>,
    }

    impl GeneratedFileWriter for RecordingGeneratedFileWriter<'_> {
        fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()> {
            *self.written.borrow_mut() = Some(files.clone());

            Ok(())
        }
    }

    impl GeneratedFileCleaner for RecordingGeneratedFileWriter<'_> {
        fn clean_stale(
            &self,
            output_dir: &Path,
            current_files: &core::GeneratedFiles,
        ) -> core::DiagnosticResult<()> {
            *self.cleaned.borrow_mut() = Some((output_dir.to_path_buf(), current_files.clone()));

            Ok(())
        }
    }
}
