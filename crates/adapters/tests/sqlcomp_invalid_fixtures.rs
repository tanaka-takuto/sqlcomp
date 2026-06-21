use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlcomp_adapters::source_fs::{FileSystemSourceReader, split_sqlcomp_query_blocks};
use sqlcomp_app::{
    CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase, DefaultQueryCompiler,
    DialectAnalyzer, MetadataProvider, SourceReader, TargetGenerator,
};
use sqlcomp_core as core;

const DIRECT_PARAM_SLOT_ID_COLLISION: &str =
    include_str!("../../../fixtures/sql/invalid/direct_param_slot_id_collision.sql");
const DUPLICATE_FRAGMENT_IDS: &str =
    include_str!("../../../fixtures/sql/invalid/duplicate_fragment_ids.sql");
const DUPLICATE_IDS: &str = include_str!("../../../fixtures/sql/invalid/duplicate_ids.sql");
const DUPLICATE_QUERY_FRAGMENT_ID: &str =
    include_str!("../../../fixtures/sql/invalid/duplicate_query_fragment_id.sql");
const EXEC_CARDINALITY: &str = include_str!("../../../fixtures/sql/invalid/exec_cardinality.sql");
const FRAGMENT_INVALID_ID: &str =
    include_str!("../../../fixtures/sql/invalid/fragment_invalid_id.sql");
const FRAGMENT_PARAM_SAMPLE_PLACEHOLDER: &str =
    include_str!("../../../fixtures/sql/invalid/fragment_param_sample_placeholder.sql");
const FRAGMENT_RAW_PLACEHOLDER: &str =
    include_str!("../../../fixtures/sql/invalid/fragment_raw_placeholder.sql");
const FRAGMENT_RAW_STATEMENT_SEPARATOR: &str =
    include_str!("../../../fixtures/sql/invalid/fragment_raw_statement_separator.sql");
const FRAGMENT_UNKNOWN_METADATA_FIELD: &str =
    include_str!("../../../fixtures/sql/invalid/fragment_unknown_metadata_field.sql");
const INVALID_ID: &str = include_str!("../../../fixtures/sql/invalid/invalid_id.sql");
const MULTIPLE_STATEMENTS: &str =
    include_str!("../../../fixtures/sql/invalid/multiple_statements.sql");
const NON_SELECT: &str = include_str!("../../../fixtures/sql/invalid/non_select.sql");
const PARAM_END_WITHOUT_START: &str =
    include_str!("../../../fixtures/sql/invalid/param_end_without_start.sql");
const PARAM_INVALID_ID: &str = include_str!("../../../fixtures/sql/invalid/param_invalid_id.sql");
const PARAM_MISSING_END: &str = include_str!("../../../fixtures/sql/invalid/param_missing_end.sql");
const PARAM_NESTED_RANGES: &str =
    include_str!("../../../fixtures/sql/invalid/param_nested_ranges.sql");
const PARAM_RAW_PLACEHOLDER: &str =
    include_str!("../../../fixtures/sql/invalid/param_raw_placeholder.sql");
const PARAM_SAMPLE_PLACEHOLDER: &str =
    include_str!("../../../fixtures/sql/invalid/param_sample_placeholder.sql");
const PARAM_UNSUPPORTED_VALUE_TYPE: &str =
    include_str!("../../../fixtures/sql/invalid/param_unsupported_value_type.sql");
const REPEATED_SLOT_DIFFERENT_TARGETS: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_slot_different_targets.sql");
const REPEATED_SLOT_SAME_TARGETS_DIFFERENT_ORDER: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_slot_same_targets_different_order.sql");
const SLOT_DUPLICATE_TARGET: &str =
    include_str!("../../../fixtures/sql/invalid/slot_duplicate_target.sql");
const SLOT_EMPTY_TARGETS: &str =
    include_str!("../../../fixtures/sql/invalid/slot_empty_targets.sql");
const SLOT_IN_FRAGMENT_BODY: &str =
    include_str!("../../../fixtures/sql/invalid/slot_in_fragment_body.sql");
const SLOT_NON_STRING_TARGET: &str =
    include_str!("../../../fixtures/sql/invalid/slot_non_string_target.sql");
const SLOT_UNKNOWN_METADATA_FIELD: &str =
    include_str!("../../../fixtures/sql/invalid/slot_unknown_metadata_field.sql");
const SLOT_UNKNOWN_TARGET: &str =
    include_str!("../../../fixtures/sql/invalid/slot_unknown_target.sql");
const SLOT_VARIANT_CARDINALITY_MISMATCH: &str =
    include_str!("../../../fixtures/sql/invalid/slot_variant_cardinality_mismatch.sql");
const SLOT_VARIANT_INVALID_SELECTED_FRAGMENT: &str =
    include_str!("../../../fixtures/sql/invalid/slot_variant_invalid_selected_fragment.sql");
const SLOT_VARIANT_LIMIT_EXCEEDED: &str =
    include_str!("../../../fixtures/sql/invalid/slot_variant_limit_exceeded.sql");
const TOP_LEVEL_PARAM: &str = include_str!("../../../fixtures/sql/invalid/top_level_param.sql");
const TOP_LEVEL_PARAM_END: &str =
    include_str!("../../../fixtures/sql/invalid/top_level_param_end.sql");
const TOP_LEVEL_SLOT: &str = include_str!("../../../fixtures/sql/invalid/top_level_slot.sql");

#[test]
fn invalid_metadata_fixtures_fail_during_source_intake() {
    assert_source_error_contains(INVALID_ID, "invalid query id `123bad`");
    assert_source_error_contains(
        EXEC_CARDINALITY,
        "`cardinality: exec` is reserved for future non-SELECT support",
    );
}

#[test]
fn invalid_sql_shape_fixtures_fail_during_dialect_analysis() {
    assert_analysis_error_contains(
        MULTIPLE_STATEMENTS,
        "expected exactly one SQL statement per query block; found 2",
    );
    assert_analysis_error_contains(
        NON_SELECT,
        "unsupported SQL statement `INSERT`; supported statement kind is `SELECT`",
    );
}

#[test]
fn invalid_param_source_fixtures_fail_during_source_intake() {
    assert_source_error_contains(
        PARAM_RAW_PLACEHOLDER,
        "raw `?` placeholders are not supported in source SQL; use paired `@sqlcomp` Param markers",
    );
    assert_source_error_contains(
        PARAM_SAMPLE_PLACEHOLDER,
        "`?` placeholders are not allowed inside Param sample expressions",
    );
    assert_source_error_contains(
        PARAM_MISSING_END,
        "`param` marker is missing a matching `paramEnd` marker",
    );
    assert_source_error_contains(
        PARAM_END_WITHOUT_START,
        "`paramEnd` marker has no matching `param` marker",
    );
    assert_source_error_contains(PARAM_NESTED_RANGES, "nested Param ranges are not supported");
    assert_source_error_contains(
        PARAM_INVALID_ID,
        "invalid Param id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
    );
    assert_source_error_contains(
        PARAM_UNSUPPORTED_VALUE_TYPE,
        "unsupported Param valueType `banana`",
    );
}

#[test]
fn invalid_slot_fragment_source_fixtures_fail_during_source_intake() {
    assert_source_error_contains(FRAGMENT_INVALID_ID, "invalid fragment id `123bad`");
    assert_source_error_contains(
        FRAGMENT_UNKNOWN_METADATA_FIELD,
        "unknown `fragment` metadata field `description`",
    );
    assert_source_error_contains(
        FRAGMENT_RAW_STATEMENT_SEPARATOR,
        "raw statement separator `;` is not supported in fragment bodies",
    );
    assert_source_error_contains(
        FRAGMENT_RAW_PLACEHOLDER,
        "raw `?` placeholders are not supported in source SQL; use paired `@sqlcomp` Param markers",
    );
    assert_source_error_contains(
        FRAGMENT_PARAM_SAMPLE_PLACEHOLDER,
        "`?` placeholders are not allowed inside Param sample expressions",
    );
    assert_source_error_contains(
        TOP_LEVEL_PARAM,
        "`param` markers must appear inside a query or fragment body; top-level Param markers are not supported",
    );
    assert_source_error_contains(
        TOP_LEVEL_PARAM_END,
        "`paramEnd` markers must appear inside a query or fragment body; top-level paramEnd markers are not supported",
    );
    assert_source_error_contains(
        TOP_LEVEL_SLOT,
        "`slot` markers must appear inside a query body; top-level Slot markers are not supported",
    );
    assert_source_error_contains(
        SLOT_IN_FRAGMENT_BODY,
        "slot markers inside fragments are not supported yet",
    );
    assert_source_error_contains(
        SLOT_UNKNOWN_METADATA_FIELD,
        "unknown `slot` metadata field `default`",
    );
    assert_source_error_contains(
        SLOT_EMPTY_TARGETS,
        "`slot` metadata field `targets` must contain at least one value",
    );
    assert_source_error_contains(
        SLOT_NON_STRING_TARGET,
        "`slot` metadata field `targets` must be a string array",
    );
}

#[test]
fn invalid_slot_fragment_compile_fixtures_fail_before_metadata_lookup() {
    let cases = [
        (
            "slot_duplicate_target.sql",
            SLOT_DUPLICATE_TARGET,
            "duplicate Slot target `activeOnly` in Slot `filter`",
        ),
        (
            "slot_unknown_target.sql",
            SLOT_UNKNOWN_TARGET,
            "unknown Slot target `missingFilter` in Slot `filter`; no fragment with that id was found",
        ),
        (
            "repeated_slot_different_targets.sql",
            REPEATED_SLOT_DIFFERENT_TARGETS,
            "conflicting Slot `filter` targets in query `repeatedSlotDifferentTargets`: first occurrence uses [activeOnly] but conflicting occurrence uses [textOnly]",
        ),
        (
            "repeated_slot_same_targets_different_order.sql",
            REPEATED_SLOT_SAME_TARGETS_DIFFERENT_ORDER,
            "conflicting Slot `filter` targets in query `repeatedSlotSameTargetsDifferentOrder`: first occurrence uses [activeOnly, textOnly] but conflicting occurrence uses [textOnly, activeOnly]",
        ),
        (
            "direct_param_slot_id_collision.sql",
            DIRECT_PARAM_SLOT_ID_COLLISION,
            "Slot `filter` in query `directParamSlotIdCollision` conflicts with query direct Param `filter`",
        ),
        (
            "slot_variant_limit_exceeded.sql",
            SLOT_VARIANT_LIMIT_EXCEEDED,
            "Slot expansion for query `slotVariantLimitExceeded` would produce 625 SQL variants, exceeding the 256 variant limit",
        ),
        (
            "slot_variant_invalid_selected_fragment.sql",
            SLOT_VARIANT_INVALID_SELECTED_FRAGMENT,
            "failed to parse MySQL SQL",
        ),
        (
            "slot_variant_cardinality_mismatch.sql",
            SLOT_VARIANT_CARDINALITY_MISMATCH,
            "Slot expansion variant for query `slotVariantCardinalityMismatch` resolved effective cardinality `one`, but the base variant resolved effective cardinality `many`",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_check_error_contains(file_name, source, expected);
    }

    assert_check_error_contains(
        "slot_variant_invalid_selected_fragment.sql",
        SLOT_VARIANT_INVALID_SELECTED_FRAGMENT,
        "while validating Slot expansion variant for query `slotVariantInvalidSelectedFragment` with selections: filter=invalidPredicate",
    );
}

#[test]
fn duplicate_id_fixture_fails_during_filesystem_source_read() {
    assert_source_reader_error_contains(
        "duplicate_ids.sql",
        DUPLICATE_IDS,
        "duplicate query id `duplicatedQuery`",
    );
    assert_source_reader_error_contains(
        "duplicate_fragment_ids.sql",
        DUPLICATE_FRAGMENT_IDS,
        "duplicate fragment id `duplicatedFragment`",
    );
    assert_source_reader_error_contains(
        "duplicate_query_fragment_id.sql",
        DUPLICATE_QUERY_FRAGMENT_ID,
        "duplicate source unit id `sharedSourceUnit`",
    );
}

fn assert_source_error_contains(source: &str, expected: &str) {
    let report = split_sqlcomp_query_blocks(source).expect_err("source fixture should fail");

    assert!(
        report.diagnostics()[0].message().contains(expected),
        "{}",
        report.diagnostics()[0].message()
    );
}

fn assert_analysis_error_contains(source: &str, expected: &str) {
    let queries =
        split_sqlcomp_query_blocks(source).expect("source fixture should pass source intake");
    let report = MysqlDialectAnalyzer
        .analyze(&queries[0])
        .expect_err("SQL fixture should fail dialect analysis");

    assert!(
        report.diagnostics()[0].message().contains(expected),
        "{}",
        report.diagnostics()[0].message()
    );
}

fn assert_source_reader_error_contains(file_name: &str, source: &str, expected: &str) {
    let project_dir = unique_temp_dir("sqlcomp-invalid-source-fixture");
    let invalid_dir = project_dir.join("invalid");
    std::fs::create_dir_all(&invalid_dir).expect("temp invalid dir should be created");
    std::fs::write(invalid_dir.join(file_name), source).expect("invalid fixture should be written");

    let plan = core::CompilationPlan::new(
        project_dir.clone(),
        vec![project_dir.join("invalid/**/*.sql")],
        Vec::new(),
        project_dir.join("generated"),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    );
    let report = FileSystemSourceReader
        .read(&plan)
        .expect_err("invalid source fixture should be rejected");
    let messages = diagnostic_messages(&report);

    assert!(
        messages.contains(expected),
        "expected diagnostic containing `{expected}`, got:\n{messages}"
    );

    std::fs::remove_dir_all(project_dir).expect("temp project dir should be removed");
}

fn assert_check_error_contains(file_name: &str, source: &str, expected: &str) {
    let project_dir = unique_temp_dir("sqlcomp-invalid-check-fixture");
    let invalid_dir = project_dir.join("invalid");
    std::fs::create_dir_all(&invalid_dir).expect("temp invalid dir should be created");
    std::fs::write(invalid_dir.join(file_name), source).expect("invalid fixture should be written");

    let config = core::ProjectConfig::new(
        project_dir.clone(),
        core::SourceConfig::new(vec!["invalid/**/*.sql".to_owned()], Vec::new()),
        core::OutputConfig::new("generated".to_owned()),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    );
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &FileSystemSourceReader,
        dialect_analyzer: &MysqlDialectAnalyzer,
        metadata_provider: &UnexpectedMetadataProvider,
        query_compiler: &DefaultQueryCompiler,
        target_generator: &UnexpectedTargetGenerator,
        generated_file_writer: &FileSystemGeneratedFileWriter,
    };
    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("invalid fixture should fail before generation");
    let messages = diagnostic_messages(&report);

    assert!(
        messages.contains(expected),
        "expected diagnostic containing `{expected}`, got:\n{messages}"
    );

    std::fs::remove_dir_all(project_dir).expect("temp project dir should be removed");
}

fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
    report
        .diagnostics()
        .iter()
        .map(core::Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n")
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

struct UnexpectedMetadataProvider;

impl MetadataProvider for UnexpectedMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "unexpected metadata lookup for query `{}`",
                query.metadata().id()
            ),
        )))
    }
}

struct UnexpectedTargetGenerator;

impl TargetGenerator for UnexpectedTargetGenerator {
    fn generate(
        &self,
        _plan: &core::CompilationPlan,
        _queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        Err(core::DiagnosticReport::new(core::Diagnostic::error(
            "unexpected target generation",
        )))
    }
}
