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
fn check_accepts_slotless_mutation_source_units() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES (?);".to_owned(),
    )
    .with_analysis_sql("INSERT INTO users (email) VALUES (?);".to_owned())
    .with_param_usages(vec![test_param_usage(
        "email",
        "INSERT INTO users (email) VALUES (".len(),
    )])
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("slotless mutation pipeline should run successfully");

    assert!(outcome.diagnostics().is_empty());
    assert_eq!(outcome.source_file_count(), 1);
    assert_eq!(outcome.query_count(), 0);
    assert_eq!(outcome.mutation_count(), 1);
    assert_eq!(outcome.builder_count(), 1);
    assert_eq!(outcome.unique_slot_count(), 0);
    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(
        outcome.mutation_summaries(),
        [crate::MutationSummary::new(
            "createUser".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            core::MutationKind::Insert,
            1,
            1,
            0,
            1,
        )]
    );
    assert_eq!(
        calls.entries(),
        [
            "read",
            "analyze_mutation",
            "describe_mutation",
            "compile_mutation",
            "generate"
        ]
    );
    let generated_builders = target_generator.generated_builders();
    assert_eq!(generated_builders.len(), 1);
    let core::CompiledBuilder::Mutation(compiled) = &generated_builders[0] else {
        panic!("target generator should receive a mutation builder");
    };
    assert_eq!(compiled.id().as_str(), "createUser");
    assert_eq!(compiled.kind(), core::MutationKind::Insert);
    assert_eq!(compiled.source_path(), Some(Path::new("sql/users.sql")));
    assert_eq!(compiled.sql(), "INSERT INTO users (email) VALUES (?);");
    assert_eq!(
        compiled.input(),
        [core::InputField::new(
            "email".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(
        compiled.params(),
        [core::ParamBinding::new(
            "email".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
}

#[test]
fn check_accepts_mutation_slot_variants_and_builds_dynamic_ir() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let (mutation, fragment) = mutation_slot_assignment_fixture();
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_fragments(vec![fragment])
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(2);
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("mutation Slot variants should validate successfully");

    assert_eq!(outcome.fragment_count(), 1);
    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(
        outcome.mutation_summaries(),
        [crate::MutationSummary::new(
            "renameUser".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            core::MutationKind::Update,
            2,
            2,
            1,
            2,
        )]
    );
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "UPDATE users AS u SET name = ? WHERE u.id = ?;",
            "UPDATE users AS u SET name = ?, updated_at = ? WHERE u.id = ?;",
        ]
    );
    assert_eq!(
        metadata_provider.described_sql(),
        [
            "UPDATE users AS u SET name = ? WHERE u.id = ?;",
            "UPDATE users AS u SET name = ?, updated_at = ? WHERE u.id = ?;",
        ]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [
            vec!["name".to_owned(), "id".to_owned()],
            vec!["name".to_owned(), "updatedAt".to_owned(), "id".to_owned()],
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read",
            "analyze_mutation",
            "analyze_mutation",
            "describe_mutation",
            "describe_mutation",
            "compile_mutation",
            "generate"
        ]
    );

    let generated_builders = target_generator.generated_builders();
    assert_eq!(generated_builders.len(), 1);
    let core::CompiledBuilder::Mutation(compiled) = &generated_builders[0] else {
        panic!("target generator should receive a mutation builder");
    };
    assert_rename_user_dynamic_ir(compiled);
}

#[test]
fn check_expands_query_repeat_to_two_item_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let sql = "SELECT u.id FROM users AS u WHERE u.id IN (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            placeholder,
            placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", placeholder)]),
    ])
    .with_source_path("sql/users.sql");
    let source_read = SourceRead::from_queries(vec![query]).with_source_file_count(1);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator =
        FakeTargetGenerator::new(calls.clone(), core::GeneratedFiles::new(Vec::new()));
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls);
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
        .expect("query Repeat should validate with a representative two-item expansion");

    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        ["SELECT u.id FROM users AS u WHERE u.id IN (?,?);"]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [vec!["id".to_owned(), "id".to_owned()]]
    );
}

#[test]
fn check_combines_slot_variants_with_fragment_repeat_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let query_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = query_sql.find(';').expect("Slot insertion point exists");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlay { type: slot id: filter targets: [byIds] } */;"
            .to_owned(),
    )
    .with_analysis_sql(query_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["byIds".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = "\nAND u.id IN (?)";
    let placeholder = fragment_sql.find('?').expect("Repeat placeholder exists");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byIds".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "ids".to_owned(),
            ",".to_owned(),
            placeholder,
            placeholder + 1,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("id", placeholder)]),
    ])
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query.clone()])
        .with_fragments(vec![fragment])
        .with_source_units(vec![core::RawSourceUnit::Query(query)])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone());
    let query_compiler = LoggingQueryCompiler::new(calls.clone());
    let target_generator =
        FakeTargetGenerator::new(calls.clone(), core::GeneratedFiles::new(Vec::new()));
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls);
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
        .expect("Slot-selected Fragment Repeat should validate with representative SQL");

    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.id IN (?,?);",
        ]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [Vec::<String>::new(), vec!["id".to_owned(), "id".to_owned()]]
    );
}

#[test]
fn check_expands_mutation_repeat_to_two_item_validation_sql() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let sql = "INSERT INTO users (email) VALUES (?);";
    let placeholder = sql.find('?').expect("Repeat placeholder exists");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUsers".to_owned()),
        sql.to_owned(),
    )
    .with_analysis_sql(sql.to_owned())
    .with_repeat_usages(vec![
        core::RepeatUsage::new(
            "rows".to_owned(),
            ",".to_owned(),
            placeholder - 1,
            placeholder + 2,
            core::SourceLocation::unknown(),
        )
        .with_item_param_usages(vec![test_param_usage("email", placeholder)]),
    ])
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
    let generated_file_writer = RecordingGeneratedFileWriter::new(calls);
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
        .expect("mutation Repeat should validate with a representative two-item expansion");

    assert_eq!(outcome.variant_count(), 1);
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        ["INSERT INTO users (email) VALUES (?),(?);"]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [vec!["email".to_owned(), "email".to_owned()]]
    );
}

fn mutation_slot_assignment_fixture() -> (core::RawMutation, core::RawFragment) {
    let base_sql = "UPDATE users AS u SET name = ? WHERE u.id = ?;";
    let slot_index = base_sql
        .find(" WHERE")
        .expect("Slot insertion point exists before WHERE predicate");
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ?/* @sqlay { type: slot id: assignment targets: [touchUpdatedAt] } */ WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage(
            "name",
            base_sql.find('?').expect("name Param placeholder exists"),
        ),
        test_param_usage(
            "id",
            base_sql.rfind('?').expect("id Param placeholder exists"),
        ),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "assignment".to_owned(),
        vec!["touchUpdatedAt".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = ", updated_at = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("touchUpdatedAt".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![test_param_usage(
        "updatedAt",
        fragment_sql
            .find('?')
            .expect("updatedAt Param placeholder exists"),
    )])
    .with_source_path("sql/mutation_fragments.sql");

    (mutation, fragment)
}

fn assert_rename_user_dynamic_ir(compiled: &core::CompiledMutation) {
    let dynamic = compiled
        .dynamic_body()
        .expect("Slot mutation should carry dynamic Core IR");
    assert_eq!(dynamic.base_segments().len(), 2);
    assert_eq!(
        dynamic.base_segments()[0].sql(),
        "UPDATE users AS u SET name = ?"
    );
    assert_eq!(
        dynamic.base_segments()[0].params(),
        [core::ParamBinding::new(
            "name".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(dynamic.slot_occurrences().len(), 1);
    assert_eq!(dynamic.slot_occurrences()[0].slot_id(), "assignment");
    assert_eq!(dynamic.base_segments()[1].sql(), " WHERE u.id = ?;");
    assert_eq!(
        dynamic.base_segments()[1].params(),
        [core::ParamBinding::new(
            "id".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(dynamic.slots().len(), 1);
    assert_eq!(dynamic.slots()[0].id(), "assignment");
    let branch = &dynamic.slots()[0].branches()[0];
    assert_eq!(branch.target_id(), "touchUpdatedAt");
    assert_eq!(branch.segments()[0].sql(), ", updated_at = ?");
    assert_eq!(
        branch.segments()[0].params(),
        [core::ParamBinding::new(
            "updatedAt".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
}

#[test]
fn check_rejects_mutation_slot_variant_that_changes_statement_kind() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let base_sql = "UPDATE users AS u SET name = ? WHERE u.id = ?;";
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("renameUser".to_owned()),
        "/* @sqlay { type: slot id: prefix targets: [deleteInstead] } */UPDATE users AS u SET name = ? WHERE u.id = ?;"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage(
            "name",
            base_sql
                .find('?')
                .expect("name Param placeholder exists"),
        ),
        test_param_usage(
            "id",
            base_sql
                .rfind('?')
                .expect("id Param placeholder exists"),
        ),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "prefix".to_owned(),
        vec!["deleteInstead".to_owned()],
        0,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("deleteInstead".to_owned()),
        "DELETE FROM users WHERE id = ?; ".to_owned(),
    )
    .with_analysis_sql("DELETE FROM users WHERE id = ?; ".to_owned())
    .with_param_usages(vec![test_param_usage(
        "id",
        "DELETE FROM users WHERE id = ".len(),
    )]);
    let source_read = SourceRead::from_queries(Vec::new())
        .with_mutations(vec![mutation.clone()])
        .with_fragments(vec![fragment])
        .with_source_units(vec![core::RawSourceUnit::Mutation(mutation)])
        .with_source_file_count(2);
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
        .expect_err("mutation Slot variants must keep the base statement kind");

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion variant for mutation `renameUser` resolved statement kind `DELETE`, but the base variant resolved statement kind `UPDATE`; all variants must have matching mutation statement kind\nwhile validating Slot expansion variant for mutation `renameUser` with selections: prefix=deleteInstead\nSlot `prefix` selected `deleteInstead` in this variant"
    );
    assert_eq!(
        calls.entries(),
        ["read", "analyze_mutation", "analyze_mutation"]
    );
}

#[test]
fn check_preserves_mixed_query_and_mutation_source_order_for_generation() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let query = raw_query();
    let mutation = core::RawMutation::new(
        core::MutationMetadata::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES (?);".to_owned(),
    )
    .with_analysis_sql("INSERT INTO users (email) VALUES (?);".to_owned())
    .with_param_usages(vec![test_param_usage(
        "email",
        "INSERT INTO users (email) VALUES (".len(),
    )])
    .with_source_path("sql/users.sql");
    let source_read = SourceRead::from_queries(vec![query.clone()])
        .with_mutations(vec![mutation.clone()])
        .with_source_units(vec![
            core::RawSourceUnit::Mutation(mutation),
            core::RawSourceUnit::Query(query),
        ])
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("mixed builders should compile through the app pipeline");

    assert_eq!(outcome.builder_count(), 2);
    assert_eq!(outcome.query_count(), 1);
    assert_eq!(outcome.mutation_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(
        target_generator
            .generated_builders()
            .iter()
            .map(core::CompiledBuilder::id)
            .collect::<Vec<_>>(),
        ["createUser", "listUsers"]
    );
    assert_eq!(
        calls.entries(),
        [
            "read",
            "analyze_mutation",
            "describe_mutation",
            "compile_mutation",
            "analyze",
            "describe",
            "compile",
            "generate"
        ]
    );
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
