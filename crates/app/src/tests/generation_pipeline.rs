use super::support::*;
use super::*;

#[test]
fn check_runs_full_generation_pipeline_without_writing_files() {
    let temp_dir = unique_temp_dir("sqlay-app-check-dry-run");
    std::fs::create_dir_all(&temp_dir).expect("temp project dir should be created");
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let generated_path = temp_dir.join("src/generated/sqlay/sql/users.ts");
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("check should dry-run generation successfully");

    assert!(outcome.diagnostics().is_empty());
    assert_eq!(outcome.source_file_count(), 1);
    assert_eq!(outcome.query_count(), 1);
    assert_eq!(
        outcome.output_dir(),
        Path::new("/tmp/sqlay-project/src/generated/sqlay")
    );
    assert_eq!(
        outcome.query_summaries(),
        [crate::QuerySummary::new(
            "listUsers".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            0,
            0,
            0,
            1
        )]
    );
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
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let generated_files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
        PathBuf::from("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
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
    assert_eq!(outcome.source_file_count(), 1);
    assert_eq!(outcome.query_count(), 1);
    assert_eq!(outcome.generated_file_count(), 1);
    assert_eq!(
        outcome.output_dir(),
        Path::new("/tmp/sqlay-project/src/generated/sqlay")
    );
    assert_eq!(
        outcome.generated_file_paths(),
        [PathBuf::from(
            "/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"
        )]
    );
    assert_eq!(
        outcome.query_summaries(),
        [crate::QuerySummary::new(
            "listUsers".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            0,
            0,
            0,
            1
        )]
    );
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
        let config = project_config(PathBuf::from("/tmp/sqlay-project"));
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
fn check_rejects_mutation_source_units_until_mutation_pipeline_is_implemented() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES ('ada@example.test');".to_owned(),
    )
    .with_source_path("sql/users.sql");
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(1);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator =
        FakeTargetGenerator::new(calls.clone(), core::GeneratedFiles::new(Vec::new()));
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
        .expect_err("mutation pipeline is not implemented in this slice");

    assert_eq!(
        diagnostic_messages(&report),
        "mutation source unit `createUser` is parsed by source intake, but mutation analysis and generation are not implemented yet"
    );
    assert_eq!(calls.entries(), ["read"]);
}

#[test]
fn compile_clean_writes_generated_files_and_removes_stale_files() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let generated_files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
        PathBuf::from("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
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
        PathBuf::from("/tmp/sqlay-project/src/generated/sqlay")
    );
    assert_eq!(current_files, generated_files);
}
