use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use sqlcomp_core as core;

use crate::{
    CompilationPlanner, CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase,
    DefaultQueryCompiler, DialectAnalyzer, GeneratedFileCleaner, GeneratedFileWriter,
    MetadataProvider, QueryCompiler, SourceRead, SourceReader, TargetGenerator,
};

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
fn check_runs_full_generation_pipeline_without_writing_files() {
    let temp_dir = unique_temp_dir("sqlcomp-app-check-dry-run");
    std::fs::create_dir_all(&temp_dir).expect("temp project dir should be created");
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let generated_path = temp_dir.join("src/generated/sqlcomp/sql/users.ts");
    let calls = CallLog::default();
    let source_reader = FakeSourceReader::new(calls.clone());
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator = FakeTargetGenerator::new(
        calls.clone(),
        core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            generated_path.clone(),
            "generated".to_owned(),
        )]),
    );
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls.clone());
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let diagnostics = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("check should dry-run generation successfully");

    assert!(diagnostics.is_empty());
    assert_eq!(
        calls.entries(),
        ["read", "analyze", "describe", "compile", "generate"]
    );
    assert!(
        !generated_path.exists(),
        "check must not write generated files"
    );

    std::fs::remove_dir_all(temp_dir).expect("temp project dir should be removed");
}

#[test]
fn compile_writes_generated_files_from_the_shared_pipeline() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let generated_files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
        PathBuf::from("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/users.ts"),
        "generated".to_owned(),
    )]);
    let calls = CallLog::default();
    let source_reader = FakeSourceReader::new(calls.clone());
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator = FakeTargetGenerator::new(calls.clone(), generated_files.clone());
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls.clone());
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let outcome = DefaultCompileUseCase::compile(&config, &pipeline, false)
        .expect("compile should write generated files");

    assert!(outcome.diagnostics().is_empty());
    assert_eq!(outcome.generated_file_count(), 1);
    assert_eq!(outcome.stale_file_removal_count(), None);
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "describe", "compile", "generate", "write"
        ]
    );
    assert_eq!(
        generated_file_writer.written_files(),
        generated_files.files()
    );
}

#[test]
fn check_reports_dialect_metadata_and_generation_errors_as_diagnostics() {
    let cases = [
        PipelineFailure::Dialect,
        PipelineFailure::Metadata,
        PipelineFailure::Generation,
    ];

    for failure in cases {
        let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
        let calls = CallLog::default();
        let source_reader = FakeSourceReader::new(calls.clone());
        let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone()).with_failure(failure);
        let metadata_provider = FakeMetadataProvider::new(calls.clone()).with_failure(failure);
        let query_compiler = LoggingQueryCompiler::new(calls.clone());
        let target_generator =
            FakeTargetGenerator::new(calls.clone(), core::GeneratedFiles::new(Vec::new()))
                .with_failure(failure);
        let generated_file_writer = RecordingGeneratedFileWriter::new(calls.clone());
        let pipeline = CompilePipeline {
            planner: &DefaultCompilationPlanner,
            source_reader: &source_reader,
            dialect_analyzer: &dialect_analyzer,
            metadata_provider: &metadata_provider,
            query_compiler: &query_compiler,
            target_generator: &target_generator,
            generated_file_writer: &generated_file_writer,
        };

        let report = DefaultCompileUseCase::check(&config, &pipeline)
            .expect_err("pipeline failures should be returned as diagnostics");

        assert_eq!(diagnostic_messages(&report), failure.message());
    }
}

#[test]
fn compile_clean_writes_generated_files_and_removes_stale_files() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let generated_files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
        PathBuf::from("/tmp/sqlcomp-project/src/generated/sqlcomp/sql/users.ts"),
        "generated".to_owned(),
    )]);
    let calls = CallLog::default();
    let source_reader = FakeSourceReader::new(calls.clone());
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator = FakeTargetGenerator::new(calls.clone(), generated_files.clone());
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls.clone());
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    let outcome = DefaultCompileUseCase::compile(&config, &pipeline, true)
        .expect("compile --clean should run generation and cleanup");

    assert!(outcome.diagnostics().is_empty());
    assert_eq!(outcome.generated_file_count(), 1);
    assert_eq!(outcome.stale_file_removal_count(), Some(0));
    let (output_dir, current_files) = generated_file_writer
        .cleaned_files()
        .expect("compile --clean should clean stale generated files");
    assert_eq!(
        calls.entries(),
        [
            "read",
            "analyze",
            "describe",
            "compile",
            "generate",
            "write",
            "clean_stale"
        ]
    );
    assert_eq!(
        generated_file_writer.written_files(),
        generated_files.files()
    );
    assert_eq!(
        output_dir,
        PathBuf::from("/tmp/sqlcomp-project/src/generated/sqlcomp")
    );
    assert_eq!(current_files, generated_files);
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

fn raw_query() -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT id FROM users;".to_owned(),
    )
    .with_source_path("sql/users.sql")
}

fn metadata() -> core::DbQueryMetadata {
    core::DbQueryMetadata::new(vec![core::DbResultColumn::new(
        "id".to_owned(),
        core::CoreType::Int64,
        Some(false),
    )])
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

#[derive(Clone, Debug, Default)]
struct CallLog(Rc<RefCell<Vec<&'static str>>>);

impl CallLog {
    fn push(&self, call: &'static str) {
        self.0.borrow_mut().push(call);
    }

    fn entries(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PipelineFailure {
    Dialect,
    Metadata,
    Generation,
}

impl PipelineFailure {
    const fn message(self) -> &'static str {
        match self {
            Self::Dialect => "dialect failed",
            Self::Metadata => "metadata failed",
            Self::Generation => "generation failed",
        }
    }

    fn report(self) -> core::DiagnosticReport {
        core::DiagnosticReport::new(core::Diagnostic::error(self.message()))
    }
}

#[derive(Clone, Debug)]
struct FakeSourceReader {
    calls: CallLog,
}

impl FakeSourceReader {
    const fn new(calls: CallLog) -> Self {
        Self { calls }
    }
}

impl SourceReader for FakeSourceReader {
    fn read(&self, _plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead> {
        self.calls.push("read");

        Ok(SourceRead::from_queries(vec![raw_query()]))
    }
}

#[derive(Clone, Debug)]
struct FakeDialectAnalyzer {
    calls: CallLog,
    failure: Option<PipelineFailure>,
}

impl FakeDialectAnalyzer {
    const fn new(calls: CallLog) -> Self {
        Self {
            calls,
            failure: None,
        }
    }

    const fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Dialect) {
            self.failure = Some(failure);
        }

        self
    }
}

impl DialectAnalyzer for FakeDialectAnalyzer {
    fn analyze(&self, _query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        self.calls.push("analyze");

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        Ok(core::AnalyzedQuery::new(core::Cardinality::Many))
    }
}

#[derive(Clone, Debug)]
struct FakeMetadataProvider {
    calls: CallLog,
    failure: Option<PipelineFailure>,
}

impl FakeMetadataProvider {
    const fn new(calls: CallLog) -> Self {
        Self {
            calls,
            failure: None,
        }
    }

    const fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Metadata) {
            self.failure = Some(failure);
        }

        self
    }
}

impl MetadataProvider for FakeMetadataProvider {
    fn describe(
        &self,
        _query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        self.calls.push("describe");

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        Ok(metadata())
    }
}

#[derive(Clone, Debug)]
struct LoggingQueryCompiler {
    calls: CallLog,
}

impl LoggingQueryCompiler {
    const fn new(calls: CallLog) -> Self {
        Self { calls }
    }
}

impl QueryCompiler for LoggingQueryCompiler {
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery> {
        self.calls.push("compile");

        DefaultQueryCompiler.compile(query, analysis, metadata)
    }
}

#[derive(Clone, Debug)]
struct FakeTargetGenerator {
    calls: CallLog,
    files: core::GeneratedFiles,
    failure: Option<PipelineFailure>,
}

impl FakeTargetGenerator {
    const fn new(calls: CallLog, files: core::GeneratedFiles) -> Self {
        Self {
            calls,
            files,
            failure: None,
        }
    }

    const fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Generation) {
            self.failure = Some(failure);
        }

        self
    }
}

impl TargetGenerator for FakeTargetGenerator {
    fn generate(
        &self,
        _plan: &core::CompilationPlan,
        _queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        self.calls.push("generate");

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        Ok(self.files.clone())
    }
}

#[derive(Clone, Debug)]
struct RecordingGeneratedFileWriter {
    calls: CallLog,
    files: Rc<RefCell<Vec<core::GeneratedFile>>>,
    cleaned: Rc<RefCell<Option<(PathBuf, core::GeneratedFiles)>>>,
}

impl RecordingGeneratedFileWriter {
    fn new(calls: CallLog) -> Self {
        Self {
            calls,
            files: Rc::new(RefCell::new(Vec::new())),
            cleaned: Rc::new(RefCell::new(None)),
        }
    }

    fn written_files(&self) -> Vec<core::GeneratedFile> {
        self.files.borrow().clone()
    }

    fn cleaned_files(&self) -> Option<(PathBuf, core::GeneratedFiles)> {
        self.cleaned.borrow().clone()
    }
}

impl GeneratedFileWriter for RecordingGeneratedFileWriter {
    fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()> {
        self.calls.push("write");
        self.files.borrow_mut().extend_from_slice(files.files());

        Ok(())
    }
}

impl GeneratedFileCleaner for RecordingGeneratedFileWriter {
    fn clean_stale(
        &self,
        output_dir: &Path,
        current_files: &core::GeneratedFiles,
    ) -> core::DiagnosticResult<usize> {
        self.calls.push("clean_stale");
        *self.cleaned.borrow_mut() = Some((output_dir.to_path_buf(), current_files.clone()));

        Ok(0)
    }
}
