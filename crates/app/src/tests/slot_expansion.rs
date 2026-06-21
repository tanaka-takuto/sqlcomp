use super::support::*;
use super::*;

#[test]
fn check_validates_slot_sql_with_empty_and_selected_fragment_replacements() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["activeOnly".to_owned()],
        "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\n-- keep this ordinary SQL comment\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\n-- keep this ordinary SQL comment\nAND u.active = 1".to_owned())
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
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
        .expect("slot SQL variants should validate successfully");

    assert_eq!(outcome.fragment_count(), 1);
    assert_eq!(outcome.unique_slot_count(), 1);
    assert_eq!(outcome.variant_count(), 2);
    assert_eq!(
        outcome.query_summaries(),
        [crate::QuerySummary::new(
            "listUsers".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            0,
            0,
            1,
            2
        )]
    );
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\n-- keep this ordinary SQL comment\nAND u.active = 1;",
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "describe", "describe", "compile", "generate"
        ]
    );
}

#[test]
fn check_reports_token_adjacent_slot_replacement_from_dialect_validation() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["activeOnly".to_owned()],
        "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "AND u.active = 1".to_owned(),
    )
    .with_analysis_sql("AND u.active = 1".to_owned())
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer =
        FakeDialectAnalyzer::new(calls.clone()).with_sql_failure("1AND u.active");
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
        .expect_err("token-adjacent selected slot SQL should fail dialect validation");

    assert_eq!(
        diagnostic_messages(&report),
        "failed to parse MySQL SQL: token-adjacent slot replacement\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=activeOnly\nSlot `filter` selected `activeOnly` in this variant"
    );
    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1;",
            "SELECT u.id FROM users AS u WHERE 1 = 1AND u.active = 1;",
        ]
    );
    assert_eq!(calls.entries(), ["read", "analyze", "analyze"]);
}

#[test]
fn check_rejects_unknown_slot_target_with_slot_context() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [missingFilter] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["missingFilter".to_owned()],
        "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let source_read = SourceRead::from_queries(vec![query]).with_source_file_count(1);
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
        .expect_err("unknown Slot targets should be rejected before dialect validation");

    assert_eq!(
        diagnostic_messages(&report),
        "unknown Slot target `missingFilter` in Slot `filter`; no fragment with that id was found"
    );
    assert_eq!(calls.entries(), ["read"]);
}

#[test]
fn check_rejects_duplicate_targets_within_one_slot() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly, activeOnly] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["activeOnly".to_owned(), "activeOnly".to_owned()],
        "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned())
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
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
        .expect_err("duplicate Slot targets should be rejected before variant validation");

    assert_eq!(
        diagnostic_messages(&report),
        "duplicate Slot target `activeOnly` in Slot `filter`; each target must appear at most once in `targets`"
    );
    assert_eq!(calls.entries(), ["read"]);
}

#[test]
fn check_preserves_slot_target_order_across_fragment_files() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly, byEmail] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["activeOnly".to_owned(), "byEmail".to_owned()],
        "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let active_only = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned())
    .with_source_path("sql/fragments/active.sql");
    let by_email = core::RawFragment::new(
        core::FragmentMetadata::new("byEmail".to_owned()),
        "\nAND u.email IS NOT NULL".to_owned(),
    )
    .with_analysis_sql("\nAND u.email IS NOT NULL".to_owned())
    .with_source_path("sql/fragments/email.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![by_email, active_only])
        .with_source_file_count(3);
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("slot target resolution should work across included files");

    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.active = 1;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.email IS NOT NULL;",
        ]
    );
}

#[test]
fn check_enumerates_multiple_slot_expansion_variants_in_stable_order() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query_prefix = "SELECT u.id FROM users AS u WHERE 1 = 1";
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: zFilter targets: [activeOnly, byEmail] } *//* @sqlcomp { type: slot id: aTenant targets: [tenantOnly] } */ ORDER BY u.id;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1 ORDER BY u.id;".to_owned())
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "zFilter".to_owned(),
            vec!["activeOnly".to_owned(), "byEmail".to_owned()],
            query_prefix.len(),
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "aTenant".to_owned(),
            vec!["tenantOnly".to_owned()],
            query_prefix.len(),
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql");
    let tenant_only = core::RawFragment::new(
        core::FragmentMetadata::new("tenantOnly".to_owned()),
        "\nAND u.tenant_id = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.tenant_id = 1".to_owned());
    let by_email = core::RawFragment::new(
        core::FragmentMetadata::new("byEmail".to_owned()),
        "\nAND u.email IS NOT NULL".to_owned(),
    )
    .with_analysis_sql("\nAND u.email IS NOT NULL".to_owned());
    let active_only = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned());
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![tenant_only, by_email, active_only])
        .with_source_file_count(4);
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("all slot expansion variants should validate in stable order");

    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1 ORDER BY u.id;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.tenant_id = 1 ORDER BY u.id;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.active = 1 ORDER BY u.id;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.active = 1\nAND u.tenant_id = 1 ORDER BY u.id;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.email IS NOT NULL ORDER BY u.id;",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.email IS NOT NULL\nAND u.tenant_id = 1 ORDER BY u.id;",
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "analyze", "analyze", "analyze", "analyze", "describe",
            "describe", "describe", "describe", "describe", "describe", "compile", "generate"
        ]
    );
}

#[test]
fn check_reuses_repeated_slot_id_selection_at_each_occurrence() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let first_insertion = "SELECT u.id FROM users AS u WHERE 1 = 1".len();
    let second_insertion =
        "SELECT u.id FROM users AS u WHERE 1 = 1 AND EXISTS (SELECT 1 FROM user_roles AS ur WHERE ur.user_id = u.id"
            .len();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: userFilter targets: [activeUser] } */ AND EXISTS (SELECT 1 FROM user_roles AS ur WHERE ur.user_id = u.id/* @sqlcomp { type: slot id: userFilter targets: [activeUser] } */);".to_owned(),
    )
    .with_analysis_sql(
        "SELECT u.id FROM users AS u WHERE 1 = 1 AND EXISTS (SELECT 1 FROM user_roles AS ur WHERE ur.user_id = u.id);"
            .to_owned(),
    )
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "userFilter".to_owned(),
            vec!["activeUser".to_owned()],
            first_insertion,
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "userFilter".to_owned(),
            vec!["activeUser".to_owned()],
            second_insertion,
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql");
    let active_user = core::RawFragment::new(
        core::FragmentMetadata::new("activeUser".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned())
    .with_source_path("sql/fragments/users.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![active_user])
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("repeated Slot IDs with matching targets should share one selection");

    assert_eq!(
        dialect_analyzer.analyzed_sql(),
        [
            "SELECT u.id FROM users AS u WHERE 1 = 1 AND EXISTS (SELECT 1 FROM user_roles AS ur WHERE ur.user_id = u.id);",
            "SELECT u.id FROM users AS u WHERE 1 = 1\nAND u.active = 1 AND EXISTS (SELECT 1 FROM user_roles AS ur WHERE ur.user_id = u.id\nAND u.active = 1);",
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "describe", "describe", "compile", "generate"
        ]
    );
}

#[test]
fn check_describes_expanded_slot_variants_with_sql_ordered_params() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let (query, fragment) = slot_param_order_fixture();
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("expanded Slot variants should be described successfully");

    assert_eq!(
        metadata_provider.described_sql(),
        [
            "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1 AND u.email = ?;",
            "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1\nAND u.active = ? AND u.role_id = ? AND u.email = ?;",
        ]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [
            vec!["tenantId".to_owned(), "email".to_owned()],
            vec![
                "tenantId".to_owned(),
                "active".to_owned(),
                "roleId".to_owned(),
                "email".to_owned(),
            ],
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "describe", "describe", "compile", "generate"
        ]
    );
}

#[test]
fn check_passes_dynamic_slot_core_ir_to_target_generation() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let (query, fragment) = slot_param_order_fixture();
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("expanded Slot variants should compile into dynamic Core IR");

    let generated_queries = target_generator.generated_queries();
    assert_eq!(generated_queries.len(), 1);
    let compiled = &generated_queries[0];
    let dynamic = compiled
        .dynamic_body()
        .expect("Slot query should carry dynamic Core IR");
    assert_eq!(dynamic.base_segments().len(), 2);
    assert_eq!(
        dynamic.base_segments()[0].sql(),
        "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1"
    );
    assert_eq!(
        dynamic.base_segments()[0].params(),
        [core::ParamBinding::new(
            "tenantId".to_owned(),
            core::CoreType::String,
            false
        )]
    );
    assert_eq!(dynamic.slot_occurrences().len(), 1);
    assert_eq!(dynamic.slot_occurrences()[0].slot_id(), "filter");
    assert_eq!(dynamic.base_segments()[1].sql(), " AND u.email = ?;");
    assert_eq!(
        dynamic.base_segments()[1].params(),
        [core::ParamBinding::new(
            "email".to_owned(),
            core::CoreType::String,
            false
        )]
    );
    assert_eq!(dynamic.slots().len(), 1);
    assert_eq!(dynamic.slots()[0].id(), "filter");
    assert_eq!(dynamic.slots()[0].branches().len(), 1);
    let branch = &dynamic.slots()[0].branches()[0];
    assert_eq!(branch.target_id(), "activeAndRole");
    assert_eq!(branch.segments().len(), 1);
    assert_eq!(
        branch.segments()[0].sql(),
        "\nAND u.active = ? AND u.role_id = ?"
    );
    assert_eq!(
        branch.segments()[0].params(),
        [
            core::ParamBinding::new("active".to_owned(), core::CoreType::String, false),
            core::ParamBinding::new("roleId".to_owned(), core::CoreType::String, false),
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "describe", "describe", "compile", "generate"
        ]
    );
}

#[test]
fn check_passes_repeated_slot_occurrences_to_dynamic_core_ir() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let (query, fragment) = repeated_slot_dynamic_ir_fixture();
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("repeated Slot occurrences should compile into dynamic Core IR");

    let generated_queries = target_generator.generated_queries();
    assert_eq!(generated_queries.len(), 1);
    let dynamic = generated_queries[0]
        .dynamic_body()
        .expect("repeated Slot query should carry dynamic Core IR");
    assert_eq!(dynamic.base_segments().len(), 3);
    assert_eq!(
        dynamic.base_segments()[0].sql(),
        "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1"
    );
    assert_eq!(
        dynamic.base_segments()[0].params(),
        [core::ParamBinding::new(
            "tenantId".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(
        dynamic.base_segments()[1].sql(),
        " AND EXISTS (SELECT 1 FROM users AS ux WHERE ux.id = u.id"
    );
    assert!(dynamic.base_segments()[1].params().is_empty());
    assert_eq!(dynamic.base_segments()[2].sql(), ");");
    assert_eq!(dynamic.slot_occurrences().len(), 2);
    assert_eq!(dynamic.slot_occurrences()[0].slot_id(), "filter");
    assert_eq!(dynamic.slot_occurrences()[1].slot_id(), "filter");
    assert_eq!(dynamic.slots().len(), 1);
    assert_eq!(dynamic.slots()[0].id(), "filter");
    assert_eq!(dynamic.slots()[0].branches().len(), 1);
    let branch = &dynamic.slots()[0].branches()[0];
    assert_eq!(branch.target_id(), "activeOnly");
    assert_eq!(branch.segments().len(), 1);
    assert_eq!(branch.segments()[0].sql(), "\nAND u.active = ?");
    assert_eq!(
        branch.segments()[0].params(),
        [core::ParamBinding::new(
            "active".to_owned(),
            core::CoreType::String,
            false,
        )]
    );
    assert_eq!(
        metadata_provider.described_param_ids(),
        [
            vec!["tenantId".to_owned()],
            vec![
                "tenantId".to_owned(),
                "active".to_owned(),
                "active".to_owned(),
            ],
        ]
    );
    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "describe", "describe", "compile", "generate"
        ]
    );
}
