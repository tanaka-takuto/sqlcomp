use super::support::*;
use super::*;

#[test]
fn check_allows_same_fragment_param_id_with_different_types_in_different_slot_scopes() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let base_sql = "SELECT u.id FROM users AS u WHERE EXISTS (SELECT 1 FROM users AS x WHERE x.id = u.id) OR EXISTS (SELECT 1 FROM accounts AS x WHERE x.user_id = u.id);";
    let user_slot_index = base_sql
        .find(") OR EXISTS")
        .expect("user Slot insertion point exists before first EXISTS closes");
    let account_slot_index = base_sql
        .find(");")
        .expect("account Slot insertion point exists before statement terminator");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE EXISTS (SELECT 1 FROM users AS x WHERE x.id = u.id/* @sqlay { type: slot id: userKind targets: [byKind] } */) OR EXISTS (SELECT 1 FROM accounts AS x WHERE x.user_id = u.id/* @sqlay { type: slot id: accountKind targets: [byKind] } */);"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "userKind".to_owned(),
            vec!["byKind".to_owned()],
            user_slot_index,
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "accountKind".to_owned(),
            vec!["byKind".to_owned()],
            account_slot_index,
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql");
    let fragment_sql = " AND x.kind = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byKind".to_owned()),
        " AND x.kind = /* @sqlay { type: param id: kind } */ 'sample' /* @sqlay { type: paramEnd } */".to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![test_param_usage(
        "kind",
        fragment_sql
            .find('?')
            .expect("fragment Param placeholder exists"),
    )])
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone())
        .with_param_type_before_placeholder(
            "FROM users AS x WHERE x.id = u.id AND x.kind = ",
            core::CoreType::String,
        )
        .with_param_type_before_placeholder(
            "FROM accounts AS x WHERE x.user_id = u.id AND x.kind = ",
            core::CoreType::Int64,
        );
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
        .expect("fragment Param IDs are scoped by selected Slot branch");

    assert_eq!(
        metadata_provider.described_param_ids(),
        [
            Vec::<String>::new(),
            vec!["kind".to_owned()],
            vec!["kind".to_owned()],
            vec!["kind".to_owned(), "kind".to_owned()],
        ]
    );
}

#[test]
fn check_rejects_fragment_param_type_conflicts_across_repeated_slot_occurrences() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let base_sql = "SELECT u.id FROM users AS u WHERE EXISTS (SELECT 1 FROM users AS x WHERE x.id = u.id) OR EXISTS (SELECT 1 FROM accounts AS x WHERE x.user_id = u.id);";
    let user_slot_index = base_sql
        .find(") OR EXISTS")
        .expect("first Slot insertion point exists before first EXISTS closes");
    let account_slot_index = base_sql
        .find(");")
        .expect("second Slot insertion point exists before statement terminator");
    let first_slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(8, 88).expect("test position should be valid"),
    );
    let second_slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(9, 96).expect("test position should be valid"),
    );
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE EXISTS (SELECT 1 FROM users AS x WHERE x.id = u.id/* @sqlay { type: slot id: filter targets: [byKind] } */) OR EXISTS (SELECT 1 FROM accounts AS x WHERE x.user_id = u.id/* @sqlay { type: slot id: filter targets: [byKind] } */);"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "filter".to_owned(),
            vec!["byKind".to_owned()],
            user_slot_index,
            first_slot_location,
        ),
        core::SlotUsage::new(
            "filter".to_owned(),
            vec!["byKind".to_owned()],
            account_slot_index,
            second_slot_location,
        ),
    ])
    .with_source_path("sql/users.sql");
    let fragment_sql = " AND x.kind = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byKind".to_owned()),
        " AND x.kind = /* @sqlay { type: param id: kind } */ 'sample' /* @sqlay { type: paramEnd } */".to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![test_param_usage(
        "kind",
        fragment_sql
            .find('?')
            .expect("fragment Param placeholder exists"),
    )])
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone())
        .with_param_type_before_placeholder(
            "FROM users AS x WHERE x.id = u.id AND x.kind = ",
            core::CoreType::String,
        )
        .with_param_type_before_placeholder(
            "FROM accounts AS x WHERE x.user_id = u.id AND x.kind = ",
            core::CoreType::Int64,
        );
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

    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("same repeated Slot branch Param conflicts should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Fragment Param `kind` type in query `listUsers`, Slot `filter`, Fragment `byKind`: occurrence 1 resolved to String but occurrence 2 resolved to Int64; repeated Slot occurrences that select the same Fragment must resolve matching Param type and nullability\nfirst occurrence of Slot `filter` selecting Fragment `byKind` is here\nconflicting occurrence of Slot `filter` selecting Fragment `byKind` is here\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=byKind\nSlot `filter` selected `byKind` in this variant"
    );
}

#[test]
fn check_rejects_fragment_param_type_conflicts_across_slot_variants() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let base_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = base_sql
        .find(';')
        .expect("Slot insertion point exists before statement terminator");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlay { type: slot id: mode targets: [numericContext] } *//* @sqlay { type: slot id: kindFilter targets: [byKind] } */;".to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "mode".to_owned(),
            vec!["numericContext".to_owned()],
            slot_index,
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "kindFilter".to_owned(),
            vec!["byKind".to_owned()],
            slot_index,
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql");
    let numeric_context = core::RawFragment::new(
        core::FragmentMetadata::new("numericContext".to_owned()),
        " /* numeric context */".to_owned(),
    )
    .with_analysis_sql(" /* numeric context */".to_owned())
    .with_source_path("sql/fragments.sql");
    let by_kind_sql = " AND x.kind = ?";
    let by_kind = core::RawFragment::new(
        core::FragmentMetadata::new("byKind".to_owned()),
        " AND x.kind = /* @sqlay { type: param id: kind } */ 'sample' /* @sqlay { type: paramEnd } */".to_owned(),
    )
    .with_analysis_sql(by_kind_sql.to_owned())
    .with_param_usages(vec![test_param_usage(
        "kind",
        by_kind_sql
            .find('?')
            .expect("fragment Param placeholder exists"),
    )])
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![numeric_context, by_kind])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone())
        .with_param_type_before_placeholder("WHERE 1 = 1 AND x.kind = ", core::CoreType::String)
        .with_param_type_before_placeholder(
            "WHERE 1 = 1 /* numeric context */ AND x.kind = ",
            core::CoreType::Int64,
        );
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

    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("same slot branch Param type conflicts across variants should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `kind` types: first occurrence resolved to String but later occurrence resolved to Int64\nwhile validating Slot expansion variant for query `listUsers` with selections: mode=numericContext, kindFilter=byKind\nSlot `mode` selected `numericContext` in this variant\nSlot `kindFilter` selected `byKind` in this variant"
    );
}

#[test]
fn check_reports_fragment_param_metadata_errors_with_fragment_location_and_slot_context() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let query_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = query_sql
        .find(';')
        .expect("Slot insertion point exists before statement terminator");
    let slot_location = core::SourceLocation::at_position(
        "sql/users.sql",
        core::SourcePosition::one_based(6, 42).expect("test position should be valid"),
    );
    let fragment_location = core::SourceLocation::at_position(
        "sql/fragments.sql",
        core::SourcePosition::one_based(2, 1).expect("test position should be valid"),
    );
    let fragment_param_location = core::SourceLocation::at_position(
        "sql/fragments.sql",
        core::SourcePosition::one_based(3, 17).expect("test position should be valid"),
    );
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlay { type: slot id: filter targets: [byEmail] } */;".to_owned(),
    )
    .with_analysis_sql(query_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["byEmail".to_owned()],
        slot_index,
        slot_location,
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = " AND COALESCE(?, u.email) = u.email";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byEmail".to_owned()),
        " AND COALESCE(/* @sqlay { type: param id: email } */ 'ada@example.test' /* @sqlay { type: paramEnd } */, u.email) = u.email".to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![
        core::ParamUsage::new("email".to_owned(), None, false, fragment_param_location.clone())
            .with_placeholder_index(
                fragment_sql
                    .find('?')
                    .expect("fragment Param placeholder exists"),
            ),
    ])
    .with_source_path("sql/fragments.sql")
    .with_source_location(fragment_location);
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone());
    let metadata_provider = FakeMetadataProvider::new(calls.clone()).with_param_failure(
        "email",
        "Param `email` requires `valueType` because no supported qualified column context was found",
    );
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

    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("fragment Param metadata failures should include Slot context");

    assert_eq!(
        report.diagnostics()[0].location(),
        Some(&fragment_param_location)
    );
    assert_eq!(
        diagnostic_messages(&report),
        "Param `email` requires `valueType` because no supported qualified column context was found\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=byEmail\nSlot `filter` selected `byEmail` in this variant\nselected fragment `byEmail` is defined here"
    );
}

#[test]
fn check_rejects_param_type_conflicts_within_same_selected_fragment_scope() {
    let config = project_config(PathBuf::from("/tmp/sqlay-project"));
    let calls = CallLog::default();
    let base_sql = "SELECT u.id FROM users AS u WHERE 1 = 1;";
    let slot_index = base_sql
        .find(';')
        .expect("Slot insertion point exists before statement terminator");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlay { type: slot id: extraFilter targets: [byId] } */;".to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "extraFilter".to_owned(),
        vec!["byId".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = " AND u.email = ? AND u.id = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byId".to_owned()),
        " AND u.email = /* @sqlay { type: param id: filter valueType: string } */ 'ada@example.test' /* @sqlay { type: paramEnd } */ AND u.id = /* @sqlay { type: param id: filter valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */".to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![
        core::ParamUsage::new(
            "filter".to_owned(),
            Some(core::CoreType::String),
            false,
            core::SourceLocation::unknown(),
        )
        .with_placeholder_index(
            fragment_sql
                .find('?')
                .expect("first fragment Param placeholder exists"),
        ),
        core::ParamUsage::new(
            "filter".to_owned(),
            Some(core::CoreType::Int64),
            false,
            core::SourceLocation::unknown(),
        )
        .with_placeholder_index(
            fragment_sql
                .rfind('?')
                .expect("second fragment Param placeholder exists"),
        ),
    ]);
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
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

    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("same fragment scope Param type conflicts should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `filter` types: first occurrence resolved to String but later occurrence resolved to Int64\nwhile validating Slot expansion variant for query `listUsers` with selections: extraFilter=byId\nSlot `extraFilter` selected `byId` in this variant"
    );
}
