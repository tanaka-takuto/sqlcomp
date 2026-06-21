use super::support::*;
use super::*;

#[test]
fn check_rejects_slot_variant_cardinality_mismatch_without_override() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let slot_index = "SELECT u.id FROM users AS u WHERE 1 = 1".len();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: limiter targets: [limitOne] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "limiter".to_owned(),
        vec!["limitOne".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("limitOne".to_owned()),
        "\nLIMIT 1".to_owned(),
    )
    .with_analysis_sql("\nLIMIT 1".to_owned())
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone()).with_limit_one_inference();
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
        .expect_err("cardinality-changing Slot variants should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion variant for query `listUsers` resolved effective cardinality `one`, but the base variant resolved effective cardinality `many`; all variants must have matching effective cardinality, using an explicit query metadata `cardinality` override when present and dialect analysis otherwise\nwhile validating Slot expansion variant for query `listUsers` with selections: limiter=limitOne\nSlot `limiter` selected `limitOne` in this variant"
    );
    assert_eq!(calls.entries(), ["read", "analyze", "analyze"]);
}

#[test]
fn check_applies_explicit_cardinality_override_before_slot_variant_comparison() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let slot_index = "SELECT u.id FROM users AS u WHERE 1 = 1".len();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), Some(core::Cardinality::Many)),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: limiter targets: [limitOne] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "limiter".to_owned(),
        vec!["limitOne".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("limitOne".to_owned()),
        "\nLIMIT 1".to_owned(),
    )
    .with_analysis_sql("\nLIMIT 1".to_owned())
    .with_source_path("sql/fragments.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![fragment])
        .with_source_file_count(2);
    let source_reader = FakeSourceReader::new(calls.clone()).with_source_read(source_read);
    let dialect_analyzer = FakeDialectAnalyzer::new(calls.clone()).with_limit_one_inference();
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
        .expect("explicit cardinality override should stabilize Slot variants");

    assert_eq!(
        calls.entries(),
        [
            "read", "analyze", "analyze", "describe", "describe", "compile", "generate"
        ]
    );
}

#[test]
fn check_rejects_slot_variant_row_shape_column_count_mismatch() {
    let (report, calls) = row_shape_mismatch_report(vec![
        core::DbResultColumn::new("id".to_owned(), core::CoreType::Int64, Some(false)),
        core::DbResultColumn::new("email".to_owned(), core::CoreType::String, Some(false)),
    ]);

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion variant for query `listUsers` returned 2 result columns, but the base variant returned 1; all variants must have matching result row shape\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=shapeChanger\nSlot `filter` selected `shapeChanger` in this variant"
    );
    assert_eq!(
        calls,
        ["read", "analyze", "analyze", "describe", "describe"]
    );
}

#[test]
fn check_rejects_slot_variant_row_shape_column_name_mismatch() {
    let (report, calls) = row_shape_mismatch_report(vec![core::DbResultColumn::new(
        "user_id".to_owned(),
        core::CoreType::Int64,
        Some(false),
    )]);

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion variant for query `listUsers` result column 1 name `user_id` does not match base column name `id`; all variants must have matching result row shape\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=shapeChanger\nSlot `filter` selected `shapeChanger` in this variant"
    );
    assert_eq!(
        calls,
        ["read", "analyze", "analyze", "describe", "describe"]
    );
}

#[test]
fn check_rejects_slot_variant_row_shape_core_type_mismatch() {
    let (report, calls) = row_shape_mismatch_report(vec![core::DbResultColumn::new(
        "id".to_owned(),
        core::CoreType::String,
        Some(false),
    )]);

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion variant for query `listUsers` result column 1 CoreType `String` does not match base CoreType `Int64`; all variants must have matching result row shape\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=shapeChanger\nSlot `filter` selected `shapeChanger` in this variant"
    );
    assert_eq!(
        calls,
        ["read", "analyze", "analyze", "describe", "describe"]
    );
}

#[test]
fn check_rejects_slot_variant_row_shape_nullability_mismatch() {
    let (report, calls) = row_shape_mismatch_report(vec![core::DbResultColumn::new(
        "id".to_owned(),
        core::CoreType::Int64,
        Some(true),
    )]);

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion variant for query `listUsers` result column 1 nullability `nullable` does not match base nullability `not nullable`; all variants must have matching result row shape\nwhile validating Slot expansion variant for query `listUsers` with selections: filter=shapeChanger\nSlot `filter` selected `shapeChanger` in this variant"
    );
    assert_eq!(
        calls,
        ["read", "analyze", "analyze", "describe", "describe"]
    );
}

#[test]
fn check_rejects_repeated_slot_id_with_different_target_order() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly, byEmail] } *//* @sqlcomp { type: slot id: filter targets: [byEmail, activeOnly] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![
        core::SlotUsage::new(
            "filter".to_owned(),
            vec!["activeOnly".to_owned(), "byEmail".to_owned()],
            "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
            core::SourceLocation::unknown(),
        ),
        core::SlotUsage::new(
            "filter".to_owned(),
            vec!["byEmail".to_owned(), "activeOnly".to_owned()],
            "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
            core::SourceLocation::unknown(),
        ),
    ])
    .with_source_path("sql/users.sql");
    let active_only = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned());
    let by_email = core::RawFragment::new(
        core::FragmentMetadata::new("byEmail".to_owned()),
        "\nAND u.email IS NOT NULL".to_owned(),
    )
    .with_analysis_sql("\nAND u.email IS NOT NULL".to_owned());
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![active_only, by_email])
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
        .expect_err("repeated Slot IDs with different target order should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Slot `filter` targets in query `listUsers`: first occurrence uses [activeOnly, byEmail] but conflicting occurrence uses [byEmail, activeOnly]; repeated Slot IDs must use the same `targets` values in the same order"
    );
    assert_eq!(calls.entries(), ["read"]);
}

#[test]
fn check_rejects_slot_id_collision_with_query_direct_param_id() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE u.email = ?/* @sqlcomp { type: slot id: email targets: [activeOnly] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE u.email = ?;".to_owned())
    .with_param_usages(vec![core::ParamUsage::new(
        "email".to_owned(),
        None,
        false,
        core::SourceLocation::unknown(),
    )])
    .with_slot_usages(vec![core::SlotUsage::new(
        "email".to_owned(),
        vec!["activeOnly".to_owned()],
        "SELECT u.id FROM users AS u WHERE u.email = ?".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let active_only = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned());
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![active_only])
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
        .expect_err("Slot IDs must not collide with query direct Param IDs");

    assert_eq!(
        diagnostic_messages(&report),
        "Slot `email` in query `listUsers` conflicts with query direct Param `email`; query direct Param IDs and Slot IDs share the generated input namespace"
    );
    assert_eq!(calls.entries(), ["read"]);
}

#[test]
fn check_warns_for_unused_fragments() {
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
    let active_only = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1".to_owned(),
    )
    .with_analysis_sql("\nAND u.active = 1".to_owned())
    .with_source_path("sql/fragments/active.sql");
    let unused = core::RawFragment::new(
        core::FragmentMetadata::new("unusedFilter".to_owned()),
        "\nAND u.deleted_at IS NULL".to_owned(),
    )
    .with_analysis_sql("\nAND u.deleted_at IS NULL".to_owned())
    .with_source_path("sql/fragments/unused.sql");
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(vec![active_only, unused])
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("unused fragments should produce non-fatal diagnostics");

    assert_eq!(outcome.diagnostics().diagnostics().len(), 1);
    assert_eq!(
        outcome.diagnostics().diagnostics()[0].severity(),
        core::DiagnosticSeverity::Warning
    );
    assert_eq!(
        diagnostic_messages(outcome.diagnostics()),
        "unused fragment `unusedFilter`; no Slot target references this fragment"
    );
}

#[test]
fn check_rejects_slot_expansion_above_variant_limit() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let targets = (0..256)
        .map(|index| format!("fragment{index}"))
        .collect::<Vec<_>>();
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [fragment0] } */;".to_owned(),
    )
    .with_analysis_sql("SELECT u.id FROM users AS u WHERE 1 = 1;".to_owned())
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        targets.clone(),
        "SELECT u.id FROM users AS u WHERE 1 = 1".len(),
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragments = targets
        .iter()
        .map(|target| {
            core::RawFragment::new(
                core::FragmentMetadata::new(target.clone()),
                "\nAND u.active = 1".to_owned(),
            )
            .with_analysis_sql("\nAND u.active = 1".to_owned())
            .with_source_path("sql/fragments.sql")
        })
        .collect::<Vec<_>>();
    let source_read = SourceRead::from_queries(vec![query])
        .with_fragments(fragments)
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
        .expect_err("slot variant limit should be enforced before dialect validation");

    assert_eq!(
        diagnostic_messages(&report),
        "Slot expansion for query `listUsers` would produce 257 SQL variants, exceeding the 256 variant limit"
    );
    assert_eq!(calls.entries(), ["read"]);
}
