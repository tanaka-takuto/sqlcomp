use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::source_fs::{FileSystemSourceReader, split_sqlcomp_query_blocks};
use sqlcomp_app::{DialectAnalyzer, SourceReader};
use sqlcomp_core as core;

const DUPLICATE_IDS: &str = include_str!("../../../fixtures/sql/invalid/duplicate_ids.sql");
const EXEC_CARDINALITY: &str = include_str!("../../../fixtures/sql/invalid/exec_cardinality.sql");
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
fn duplicate_id_fixture_fails_during_filesystem_source_read() {
    let project_dir = unique_temp_dir("sqlcomp-invalid-duplicate-ids");
    let invalid_dir = project_dir.join("invalid");
    std::fs::create_dir_all(&invalid_dir).expect("temp invalid dir should be created");
    std::fs::write(invalid_dir.join("duplicate_ids.sql"), DUPLICATE_IDS)
        .expect("duplicate id fixture should be written");

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
        .expect_err("duplicate query ids should be rejected");

    assert!(
        report.diagnostics()[0]
            .message()
            .contains("duplicate query id `duplicatedQuery`"),
        "{}",
        report.diagnostics()[0].message()
    );

    std::fs::remove_dir_all(project_dir).expect("temp project dir should be removed");
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

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}
