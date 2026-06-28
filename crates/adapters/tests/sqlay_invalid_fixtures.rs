use sqlay_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlay_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlay_adapters::source_fs::{FileSystemSourceReader, split_sqlay_query_blocks};
use sqlay_app::{
    CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase, DefaultQueryCompiler,
    DialectAnalyzer, MetadataProvider, MutationMetadataProvider, SourceReader, TargetGenerator,
};
use sqlay_core as core;

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
const MUTATION_CALL: &str = include_str!("../../../fixtures/sql/invalid/mutation_call.sql");
const MUTATION_CTE: &str = include_str!("../../../fixtures/sql/invalid/mutation_cte.sql");
const MUTATION_DDL: &str = include_str!("../../../fixtures/sql/invalid/mutation_ddl.sql");
const MUTATION_DELETE_WITHOUT_WHERE: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_delete_without_where.sql");
const MUTATION_INSERT_SELECT: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_insert_select.sql");
const MUTATION_LOAD_DATA: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_load_data.sql");
const MUTATION_MULTI_TABLE_DELETE: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_multi_table_delete.sql");
const MUTATION_MULTI_TABLE_UPDATE: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_multi_table_update.sql");
const MUTATION_MULTIPLE_STATEMENTS: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_multiple_statements.sql");
const MUTATION_RAW_PLACEHOLDER: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_raw_placeholder.sql");
const MUTATION_REPLACE_SELECT: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_replace_select.sql");
const MUTATION_TRUNCATE: &str = include_str!("../../../fixtures/sql/invalid/mutation_truncate.sql");
const MUTATION_UPDATE_WITHOUT_WHERE: &str =
    include_str!("../../../fixtures/sql/invalid/mutation_update_without_where.sql");
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
const DIRECT_PARAM_REPEAT_ID_COLLISION: &str =
    include_str!("../../../fixtures/sql/invalid/direct_param_repeat_id_collision.sql");
const FRAGMENT_PARAM_REPEAT_BRANCH_COLLISION: &str =
    include_str!("../../../fixtures/sql/invalid/fragment_param_repeat_branch_collision.sql");
const REPEATED_SLOT_DIFFERENT_TARGETS: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_slot_different_targets.sql");
const REPEATED_SLOT_SAME_TARGETS_DIFFERENT_ORDER: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_slot_same_targets_different_order.sql");
const REPEAT_END_UNKNOWN_METADATA_FIELD: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_end_unknown_metadata_field.sql");
const REPEAT_END_WITHOUT_START: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_end_without_start.sql");
const REPEAT_INSIDE_PARAM_RANGE: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_inside_param_range.sql");
const REPEAT_INVALID_ID: &str = include_str!("../../../fixtures/sql/invalid/repeat_invalid_id.sql");
const REPEAT_MISSING_END: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_missing_end.sql");
const REPEAT_MISSING_SEPARATOR: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_missing_separator.sql");
const REPEAT_NESTED_RANGES: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_nested_ranges.sql");
const REPEAT_NON_STRING_SEPARATOR: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_non_string_separator.sql");
const REPEAT_REPRESENTATIVE_INVALID_SQL: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_representative_invalid_sql.sql");
const REPEAT_UNKNOWN_METADATA_FIELD: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_unknown_metadata_field.sql");
const REPEAT_VALIDATION_CASE_LIMIT_EXCEEDED: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_validation_case_limit_exceeded.sql");
const REPEAT_WITHOUT_PARAM: &str =
    include_str!("../../../fixtures/sql/invalid/repeat_without_param.sql");
const REPEATED_REPEAT_ITEM_NULLABILITY_CONFLICT: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_repeat_item_nullability_conflict.sql");
const REPEATED_REPEAT_ITEM_SHAPE_CONFLICT: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_repeat_item_shape_conflict.sql");
const REPEATED_REPEAT_ITEM_TYPE_CONFLICT: &str =
    include_str!("../../../fixtures/sql/invalid/repeated_repeat_item_type_conflict.sql");
const SLOT_INSIDE_REPEAT_RANGE: &str =
    include_str!("../../../fixtures/sql/invalid/slot_inside_repeat_range.sql");
const SLOT_REPEAT_ID_COLLISION: &str =
    include_str!("../../../fixtures/sql/invalid/slot_repeat_id_collision.sql");
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
const TOP_LEVEL_REPEAT: &str = include_str!("../../../fixtures/sql/invalid/top_level_repeat.sql");
const TOP_LEVEL_REPEAT_END: &str =
    include_str!("../../../fixtures/sql/invalid/top_level_repeat_end.sql");
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
fn invalid_mutation_sql_shape_fixtures_fail_before_metadata_lookup() {
    let cases = [
        (
            "mutation_multi_table_update.sql",
            MUTATION_MULTI_TABLE_UPDATE,
            "unsupported multi-table UPDATE; initial mutation support only accepts single-table UPDATE",
        ),
        (
            "mutation_multi_table_delete.sql",
            MUTATION_MULTI_TABLE_DELETE,
            "unsupported multi-table DELETE; initial mutation support only accepts single-table DELETE",
        ),
        (
            "mutation_insert_select.sql",
            MUTATION_INSERT_SELECT,
            "unsupported INSERT ... SELECT; initial mutation support accepts INSERT ... VALUES and INSERT ... SET",
        ),
        (
            "mutation_replace_select.sql",
            MUTATION_REPLACE_SELECT,
            "unsupported REPLACE ... SELECT; initial mutation support accepts REPLACE ... VALUES and REPLACE ... SET",
        ),
        (
            "mutation_cte.sql",
            MUTATION_CTE,
            "unsupported mutation SQL statement `WITH`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
        ),
        (
            "mutation_call.sql",
            MUTATION_CALL,
            "unsupported mutation SQL statement `CALL`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
        ),
        (
            "mutation_load_data.sql",
            MUTATION_LOAD_DATA,
            "unsupported mutation SQL statement `LOAD DATA`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
        ),
        (
            "mutation_truncate.sql",
            MUTATION_TRUNCATE,
            "unsupported mutation SQL statement `TRUNCATE`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
        ),
        (
            "mutation_ddl.sql",
            MUTATION_DDL,
            "unsupported mutation SQL statement `CREATE`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
        ),
        (
            "mutation_multiple_statements.sql",
            MUTATION_MULTIPLE_STATEMENTS,
            "expected exactly one SQL statement per mutation block; found 2",
        ),
        (
            "mutation_update_without_where.sql",
            MUTATION_UPDATE_WITHOUT_WHERE,
            "UPDATE mutation requires a WHERE clause",
        ),
        (
            "mutation_delete_without_where.sql",
            MUTATION_DELETE_WITHOUT_WHERE,
            "DELETE mutation requires a WHERE clause",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_check_error_contains(file_name, source, expected);
    }
}

#[test]
fn invalid_param_source_fixtures_fail_during_source_intake() {
    assert_source_error_contains(
        PARAM_RAW_PLACEHOLDER,
        "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers",
    );
    assert_source_error_contains(
        MUTATION_RAW_PLACEHOLDER,
        "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers",
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
        "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers",
    );
    assert_source_error_contains(
        FRAGMENT_PARAM_SAMPLE_PLACEHOLDER,
        "`?` placeholders are not allowed inside Param sample expressions",
    );
    assert_source_error_contains(
        TOP_LEVEL_PARAM,
        "`param` markers must appear inside a query, mutation, or fragment body; top-level Param markers are not supported",
    );
    assert_source_error_contains(
        TOP_LEVEL_PARAM_END,
        "`paramEnd` markers must appear inside a query, mutation, or fragment body; top-level paramEnd markers are not supported",
    );
    assert_source_error_contains(
        TOP_LEVEL_SLOT,
        "`slot` markers must appear inside a query or mutation body; top-level Slot markers are not supported",
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
            "Dynamic SQL validation for query `slotVariantLimitExceeded` would produce 625 validation cases, exceeding the 256 validation case limit",
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
fn invalid_repeat_source_fixtures_fail_during_source_intake() {
    let cases = [
        (
            REPEAT_MISSING_END,
            "`repeat` marker is missing a matching `repeatEnd` marker",
        ),
        (
            REPEAT_END_WITHOUT_START,
            "`repeatEnd` marker has no matching `repeat` marker",
        ),
        (
            REPEAT_NESTED_RANGES,
            "nested Repeat ranges are not supported",
        ),
        (
            REPEAT_INSIDE_PARAM_RANGE,
            "Repeat markers are not supported inside Param ranges",
        ),
        (
            SLOT_INSIDE_REPEAT_RANGE,
            "Slot markers are not supported inside Repeat ranges",
        ),
        (
            REPEAT_WITHOUT_PARAM,
            "Repeat ranges must contain at least one Param marker",
        ),
        (
            REPEAT_INVALID_ID,
            "invalid Repeat id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            REPEAT_MISSING_SEPARATOR,
            "missing required `repeat` metadata field `separator`",
        ),
        (
            REPEAT_NON_STRING_SEPARATOR,
            "`repeat` metadata field `separator` must be a string",
        ),
        (
            REPEAT_UNKNOWN_METADATA_FIELD,
            "unknown `repeat` metadata field `minItems`",
        ),
        (
            REPEAT_END_UNKNOWN_METADATA_FIELD,
            "unknown `repeatEnd` metadata field `id`",
        ),
        (
            TOP_LEVEL_REPEAT,
            "`repeat` markers must appear inside a query, mutation, or fragment body; top-level Repeat markers are not supported",
        ),
        (
            TOP_LEVEL_REPEAT_END,
            "`repeatEnd` markers must appear inside a query, mutation, or fragment body; top-level repeatEnd markers are not supported",
        ),
    ];

    for (source, expected) in cases {
        assert_source_error_contains(source, expected);
    }
}

#[test]
fn invalid_repeat_compile_fixtures_fail_before_metadata_lookup() {
    let cases = [
        (
            "direct_param_repeat_id_collision.sql",
            DIRECT_PARAM_REPEAT_ID_COLLISION,
            "Repeat `filter` in query `directParamRepeatIdCollision` conflicts with query direct Param `filter`",
        ),
        (
            "slot_repeat_id_collision.sql",
            SLOT_REPEAT_ID_COLLISION,
            "Repeat `filter` in query `slotRepeatIdCollision` conflicts with Slot `filter`",
        ),
        (
            "fragment_param_repeat_branch_collision.sql",
            FRAGMENT_PARAM_REPEAT_BRANCH_COLLISION,
            "Repeat `filter` in Fragment `repeatBranchCollisionFilter` selected by Slot `filter` in query `fragmentParamRepeatBranchCollision` conflicts with Fragment direct Param `filter`",
        ),
        (
            "repeated_repeat_item_shape_conflict.sql",
            REPEATED_REPEAT_ITEM_SHAPE_CONFLICT,
            "conflicting Repeat `items` item shape in query `repeatedRepeatItemShapeConflict`: first occurrence uses fields [id] but conflicting occurrence uses [id, label]",
        ),
        (
            "repeated_repeat_item_type_conflict.sql",
            REPEATED_REPEAT_ITEM_TYPE_CONFLICT,
            "conflicting Repeat `items` item shape in query `repeatedRepeatItemTypeConflict` item Param `id` type conflict",
        ),
        (
            "repeated_repeat_item_nullability_conflict.sql",
            REPEATED_REPEAT_ITEM_NULLABILITY_CONFLICT,
            "conflicting Repeat `items` item shape in query `repeatedRepeatItemNullabilityConflict` item Param `id` nullability conflict",
        ),
        (
            "repeat_representative_invalid_sql.sql",
            REPEAT_REPRESENTATIVE_INVALID_SQL,
            "failed to parse MySQL SQL",
        ),
        (
            "repeat_validation_case_limit_exceeded.sql",
            REPEAT_VALIDATION_CASE_LIMIT_EXCEEDED,
            "Dynamic SQL validation for query `repeatValidationCaseLimitExceeded` would produce 625 validation cases, exceeding the 256 validation case limit",
        ),
    ];

    for (file_name, source, expected) in cases {
        assert_check_error_contains(file_name, source, expected);
    }
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
    let report = split_sqlay_query_blocks(source).expect_err("source fixture should fail");

    assert!(
        report.diagnostics()[0].message().contains(expected),
        "{}",
        report.diagnostics()[0].message()
    );
}

fn assert_analysis_error_contains(source: &str, expected: &str) {
    let queries =
        split_sqlay_query_blocks(source).expect("source fixture should pass source intake");
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
    let project_dir = unique_temp_dir("sqlay-invalid-source-fixture");
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
    let project_dir = unique_temp_dir("sqlay-invalid-check-fixture");
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

impl MutationMetadataProvider for UnexpectedMetadataProvider {
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        _analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata> {
        Err(core::DiagnosticReport::new(core::Diagnostic::error(
            format!(
                "unexpected metadata lookup for mutation `{}`",
                mutation.metadata().id()
            ),
        )))
    }
}

struct UnexpectedTargetGenerator;

impl TargetGenerator for UnexpectedTargetGenerator {
    fn generate(
        &self,
        _plan: &core::CompilationPlan,
        _builders: &[core::CompiledBuilder],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        Err(core::DiagnosticReport::new(core::Diagnostic::error(
            "unexpected target generation",
        )))
    }
}
