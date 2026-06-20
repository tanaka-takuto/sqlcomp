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
fn source_read_carries_fragment_source_units() {
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeOnly".to_owned()),
        "\nAND u.active = 1\n".to_owned(),
    )
    .with_source_path("sql/fragments.sql");

    let source_read = SourceRead::from_queries(Vec::new()).with_fragments(vec![fragment.clone()]);

    assert!(source_read.queries().is_empty());
    assert_eq!(source_read.fragments(), [fragment]);
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

    let outcome = DefaultCompileUseCase::check(&config, &pipeline)
        .expect("check should dry-run generation successfully");

    assert!(outcome.diagnostics().is_empty());
    assert_eq!(outcome.source_file_count(), 1);
    assert_eq!(outcome.query_count(), 1);
    assert_eq!(
        outcome.output_dir(),
        Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp")
    );
    assert_eq!(
        outcome.query_summaries(),
        [crate::QuerySummary::new(
            "listUsers".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            0,
            0
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

    DefaultCompileUseCase::check(&config, &pipeline)
        .expect("slot SQL variants should validate successfully");

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
fn check_rejects_param_type_conflicts_in_selected_slot_variants() {
    let config = project_config(PathBuf::from("/tmp/sqlcomp-project"));
    let calls = CallLog::default();
    let base_sql = "SELECT u.id FROM users AS u WHERE u.email = ?;";
    let slot_index = base_sql
        .find(';')
        .expect("Slot insertion point exists before statement terminator");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE u.email = /* @sqlcomp { type: param id: filter valueType: string } */ 'ada@example.test' /* @sqlcomp { type: paramEnd } *//* @sqlcomp { type: slot id: extraFilter targets: [byId] } */;".to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_param_usages(vec![
        core::ParamUsage::new(
            "filter".to_owned(),
            Some(core::CoreType::String),
            false,
            core::SourceLocation::unknown(),
        )
        .with_placeholder_index(base_sql.find('?').expect("query Param placeholder exists")),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "extraFilter".to_owned(),
        vec!["byId".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");
    let fragment_sql = " AND u.id = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("byId".to_owned()),
        " AND u.id = /* @sqlcomp { type: param id: filter valueType: int64 } */ 1 /* @sqlcomp { type: paramEnd } */".to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![
        core::ParamUsage::new(
            "filter".to_owned(),
            Some(core::CoreType::Int64),
            false,
            core::SourceLocation::unknown(),
        )
        .with_placeholder_index(
            fragment_sql
                .find('?')
                .expect("fragment Param placeholder exists"),
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
        .expect_err("Param type conflicts in selected Slot variants should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `filter` types: first occurrence resolved to String but later occurrence resolved to Int64\nwhile validating Slot expansion variant for query `listUsers` with selections: extraFilter=byId\nSlot `extraFilter` selected `byId` in this variant"
    );
}

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
    assert_eq!(outcome.source_file_count(), 1);
    assert_eq!(outcome.query_count(), 1);
    assert_eq!(outcome.generated_file_count(), 1);
    assert_eq!(
        outcome.output_dir(),
        Path::new("/tmp/sqlcomp-project/src/generated/sqlcomp")
    );
    assert_eq!(
        outcome.generated_file_paths(),
        [PathBuf::from(
            "/tmp/sqlcomp-project/src/generated/sqlcomp/sql/users.ts"
        )]
    );
    assert_eq!(
        outcome.query_summaries(),
        [crate::QuerySummary::new(
            "listUsers".to_owned(),
            Some(PathBuf::from("sql/users.sql")),
            0,
            0
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
fn query_compiler_builds_core_ir_with_empty_paramless_input_and_result_columns() {
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
fn query_compiler_builds_input_fields_and_param_bindings_from_resolved_param_metadata() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE email = /* @sqlcomp { type: param id: email nullable: true } */ 'test@example.test' /* @sqlcomp { type: paramEnd } */ AND id = /* @sqlcomp { type: param id: userId } */ 1 /* @sqlcomp { type: paramEnd } */ OR email = /* @sqlcomp { type: param id: email nullable: true } */ 'ada@example.test' /* @sqlcomp { type: paramEnd } */;".to_owned(),
    )
    .with_analysis_sql("SELECT id FROM users WHERE email = ? AND id = ? OR email = ?;".to_owned())
    .with_param_usages(vec![
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            true,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            true,
            core::SourceLocation::unknown(),
        ),
    ]);
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        core::DbParamUsage::new("userId".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
    ]);

    let compiled = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect("resolved Param query should compile into Core IR");

    assert_eq!(
        compiled.input(),
        [
            core::InputField::new("email".to_owned(), core::CoreType::String, true),
            core::InputField::new("userId".to_owned(), core::CoreType::Int64, false),
        ]
    );
    assert_eq!(
        compiled.params(),
        [
            core::ParamBinding::new("email".to_owned(), core::CoreType::String, true),
            core::ParamBinding::new("userId".to_owned(), core::CoreType::Int64, false),
            core::ParamBinding::new("email".to_owned(), core::CoreType::String, true),
        ]
    );
}

#[test]
fn query_compiler_rejects_repeated_param_ids_with_conflicting_semantics() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE id = ? OR id = ?;".to_owned(),
    )
    .with_param_usages(vec![
        core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
    ]);
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("userId".to_owned(), core::CoreType::Int64),
        core::DbParamUsage::new("userId".to_owned(), core::CoreType::String),
    ]);

    let report = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect_err("conflicting repeated Param IDs should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `userId` types: first occurrence resolved to Int64 but later occurrence resolved to String"
    );
}

#[test]
fn query_compiler_rejects_repeated_param_ids_with_conflicting_nullability() {
    let query = core::RawQuery::new(
        core::QueryMetadata::new("findUser".to_owned(), None),
        "SELECT id FROM users WHERE email = ? OR email = ?;".to_owned(),
    )
    .with_param_usages(vec![
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        ),
        core::ParamUsage::new(
            "email".to_owned(),
            None,
            true,
            core::SourceLocation::unknown(),
        ),
    ]);
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);
    let metadata = core::DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
    ]);

    let report = DefaultQueryCompiler
        .compile(&query, &analysis, &metadata)
        .expect_err("conflicting repeated Param nullability should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        "conflicting Param `email` nullability: first occurrence is nullable false but later occurrence is nullable true"
    );
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

fn slot_param_order_fixture() -> (core::RawQuery, core::RawFragment) {
    let base_sql = "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1 AND u.email = ?;";
    let slot_index = base_sql
        .find(" AND u.email")
        .expect("Slot insertion point exists before email predicate");
    let query = core::RawQuery::new(
        core::QueryMetadata::new("listUsers".to_owned(), None),
        "SELECT u.id FROM users AS u WHERE u.tenant_id = ? AND 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeAndRole] } */ AND u.email = ?;"
            .to_owned(),
    )
    .with_analysis_sql(base_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage(
            "tenantId",
            base_sql
                .find('?')
                .expect("tenant Param placeholder exists"),
        ),
        test_param_usage("email", base_sql.rfind('?').expect("email Param placeholder exists")),
    ])
    .with_slot_usages(vec![core::SlotUsage::new(
        "filter".to_owned(),
        vec!["activeAndRole".to_owned()],
        slot_index,
        core::SourceLocation::unknown(),
    )])
    .with_source_path("sql/users.sql");

    let fragment_sql = "\nAND u.active = ? AND u.role_id = ?";
    let fragment = core::RawFragment::new(
        core::FragmentMetadata::new("activeAndRole".to_owned()),
        fragment_sql.to_owned(),
    )
    .with_analysis_sql(fragment_sql.to_owned())
    .with_param_usages(vec![
        test_param_usage(
            "active",
            fragment_sql
                .find('?')
                .expect("active Param placeholder exists"),
        ),
        test_param_usage(
            "roleId",
            fragment_sql
                .rfind('?')
                .expect("role Param placeholder exists"),
        ),
    ])
    .with_source_path("sql/fragments.sql");

    (query, fragment)
}

fn test_param_usage(id: &str, placeholder_index: usize) -> core::ParamUsage {
    core::ParamUsage::new(id.to_owned(), None, false, core::SourceLocation::unknown())
        .with_placeholder_index(placeholder_index)
}

#[test]
fn fake_dialect_analyzer_matches_limit_one_case_insensitively_and_exactly() {
    let analyzer = FakeDialectAnalyzer::new(CallLog::default()).with_limit_one_inference();

    let limit_one = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitOne".to_owned(), None),
            "SELECT id FROM users limit 1;".to_owned(),
        ))
        .expect("lowercase limit one should be analyzed");
    let limit_ten = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitTen".to_owned(), None),
            "SELECT id FROM users LIMIT 10;".to_owned(),
        ))
        .expect("limit ten should be analyzed");
    let limit_one_offset = analyzer
        .analyze(&core::RawQuery::new(
            core::QueryMetadata::new("limitOneOffset".to_owned(), None),
            "SELECT id FROM users LIMIT 1 OFFSET 5;".to_owned(),
        ))
        .expect("limit one with additional clause should be analyzed");

    assert_eq!(limit_one.cardinality(), core::Cardinality::One);
    assert_eq!(limit_ten.cardinality(), core::Cardinality::Many);
    assert_eq!(limit_one_offset.cardinality(), core::Cardinality::Many);
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
    source_read: SourceRead,
}

impl FakeSourceReader {
    fn new(calls: CallLog) -> Self {
        Self {
            calls,
            source_read: SourceRead::from_queries(vec![raw_query()]).with_source_file_count(1),
        }
    }

    fn with_source_read(mut self, source_read: SourceRead) -> Self {
        self.source_read = source_read;
        self
    }
}

impl SourceReader for FakeSourceReader {
    fn read(&self, _plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead> {
        self.calls.push("read");

        Ok(self.source_read.clone())
    }
}

#[derive(Clone, Debug)]
struct FakeDialectAnalyzer {
    calls: CallLog,
    failure: Option<PipelineFailure>,
    sql_failure: Option<&'static str>,
    infer_limit_one: bool,
    analyzed_sql: Rc<RefCell<Vec<String>>>,
}

impl FakeDialectAnalyzer {
    fn new(calls: CallLog) -> Self {
        Self {
            calls,
            failure: None,
            sql_failure: None,
            infer_limit_one: false,
            analyzed_sql: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Dialect) {
            self.failure = Some(failure);
        }

        self
    }

    const fn with_sql_failure(mut self, sql_fragment: &'static str) -> Self {
        self.sql_failure = Some(sql_fragment);
        self
    }

    const fn with_limit_one_inference(mut self) -> Self {
        self.infer_limit_one = true;
        self
    }

    fn analyzed_sql(&self) -> Vec<String> {
        self.analyzed_sql.borrow().clone()
    }
}

impl DialectAnalyzer for FakeDialectAnalyzer {
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        self.calls.push("analyze");
        self.analyzed_sql
            .borrow_mut()
            .push(query.analysis_sql().to_owned());

        if self
            .sql_failure
            .is_some_and(|sql_fragment| query.analysis_sql().contains(sql_fragment))
        {
            return Err(core::DiagnosticReport::new(core::Diagnostic::error(
                "failed to parse MySQL SQL: token-adjacent slot replacement",
            )));
        }

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        let cardinality =
            if self.infer_limit_one && analysis_sql_has_limit_one(query.analysis_sql()) {
                core::Cardinality::One
            } else {
                core::Cardinality::Many
            };

        Ok(core::AnalyzedQuery::new(cardinality))
    }
}

fn analysis_sql_has_limit_one(sql: &str) -> bool {
    let tokens = sql.split_ascii_whitespace().collect::<Vec<_>>();

    for index in 0..tokens.len().saturating_sub(1) {
        if !tokens[index].eq_ignore_ascii_case("LIMIT") {
            continue;
        }

        match tokens[index + 1] {
            "1;" => {
                return index + 2 == tokens.len();
            }
            "1" => {
                return index + 2 == tokens.len()
                    || (tokens.get(index + 2) == Some(&";") && index + 3 == tokens.len());
            }
            _ => {}
        }
    }

    false
}

#[derive(Clone, Debug)]
struct FakeMetadataProvider {
    calls: CallLog,
    failure: Option<PipelineFailure>,
    described_sql: Rc<RefCell<Vec<String>>>,
    described_param_ids: Rc<RefCell<Vec<Vec<String>>>>,
}

impl FakeMetadataProvider {
    fn new(calls: CallLog) -> Self {
        Self {
            calls,
            failure: None,
            described_sql: Rc::new(RefCell::new(Vec::new())),
            described_param_ids: Rc::new(RefCell::new(Vec::new())),
        }
    }

    const fn with_failure(mut self, failure: PipelineFailure) -> Self {
        if matches!(failure, PipelineFailure::Metadata) {
            self.failure = Some(failure);
        }

        self
    }

    fn described_sql(&self) -> Vec<String> {
        self.described_sql.borrow().clone()
    }

    fn described_param_ids(&self) -> Vec<Vec<String>> {
        self.described_param_ids.borrow().clone()
    }
}

impl MetadataProvider for FakeMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        self.calls.push("describe");
        self.described_sql
            .borrow_mut()
            .push(query.analysis_sql().to_owned());
        let param_ids = query
            .param_usages()
            .iter()
            .map(|usage| usage.id().to_owned())
            .collect::<Vec<_>>();
        self.described_param_ids.borrow_mut().push(param_ids);

        if let Some(failure) = self.failure {
            return Err(failure.report());
        }

        let param_usages = query
            .param_usages()
            .iter()
            .map(|usage| {
                core::DbParamUsage::new(
                    usage.id().to_owned(),
                    usage
                        .value_type_override()
                        .unwrap_or(core::CoreType::String),
                )
            })
            .collect();

        Ok(metadata().with_param_usages(param_usages))
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
