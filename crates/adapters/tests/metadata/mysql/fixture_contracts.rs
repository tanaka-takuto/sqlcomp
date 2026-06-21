use super::fixture_support::{
    INIT_FIXTURES, INVALID_CONFIG, QUERY_FIXTURES, VALID_CONFIG, extract_sqlcomp_queries, repo_path,
};
use super::type_coverage::{FIXTURE_ALL_COLUMN_TYPE_COVERAGE, fixture_all_column_type_columns};

#[test]
fn mysql_fixtures_use_meta_schema_names() {
    assert!(
        INIT_FIXTURES[0].contains("CREATE TABLE fixture_all_column_type"),
        "schema should use a metadata-oriented parent table name"
    );
    assert!(
        INIT_FIXTURES[0].contains("CREATE TABLE fixture_child"),
        "schema should use a metadata-oriented child table name"
    );
    assert!(
        INIT_FIXTURES[0].contains("bigint_nn_col BIGINT NOT NULL PRIMARY KEY"),
        "schema should name columns by type/nullability metadata"
    );

    for fixture in INIT_FIXTURES.iter().chain(QUERY_FIXTURES) {
        for project_term in [
            "fixture_type_metadata_users",
            "fixture_type_metadata_orders",
            "display_name",
            "nickname",
            "email",
            "order_number",
            "typeMetadataSingleUser",
            "singleUser",
        ] {
            assert!(
                !fixture.contains(project_term),
                "fixture should not contain project-like term `{project_term}`"
            );
        }
    }
}

#[test]
fn mysql_fixtures_use_sql_valid_invalid_layout() {
    for required_path in [
        "fixtures/sql/sqlcomp.valid.config.json",
        "fixtures/sql/sqlcomp.invalid.config.json",
        "fixtures/sql/valid/type_metadata_matrix.sql",
        "fixtures/sql/valid/generation_surface.sql",
        "fixtures/sql/valid/param_bindings.sql",
        "fixtures/sql/valid/slot_runtime.sql",
        "fixtures/sql/valid/nested/path_mapping.sql",
        "fixtures/sql/invalid/non_select.sql",
        "fixtures/sql/invalid/param_raw_placeholder.sql",
        "fixtures/sql/invalid/param_unsupported_inference_context.sql",
        "fixtures/sql/invalid/param_conflicting_repeated_type.sql",
        "fixtures/sql/invalid/param_conflicting_repeated_nullability.sql",
        "fixtures/sql/invalid/duplicate_fragment_ids.sql",
        "fixtures/sql/invalid/duplicate_query_fragment_id.sql",
        "fixtures/sql/invalid/fragment_invalid_id.sql",
        "fixtures/sql/invalid/fragment_unknown_metadata_field.sql",
        "fixtures/sql/invalid/fragment_raw_statement_separator.sql",
        "fixtures/sql/invalid/fragment_raw_placeholder.sql",
        "fixtures/sql/invalid/fragment_param_sample_placeholder.sql",
        "fixtures/sql/invalid/top_level_param.sql",
        "fixtures/sql/invalid/top_level_param_end.sql",
        "fixtures/sql/invalid/top_level_slot.sql",
        "fixtures/sql/invalid/slot_in_fragment_body.sql",
        "fixtures/sql/invalid/slot_unknown_metadata_field.sql",
        "fixtures/sql/invalid/slot_empty_targets.sql",
        "fixtures/sql/invalid/slot_non_string_target.sql",
        "fixtures/sql/invalid/slot_duplicate_target.sql",
        "fixtures/sql/invalid/slot_unknown_target.sql",
        "fixtures/sql/invalid/repeated_slot_different_targets.sql",
        "fixtures/sql/invalid/repeated_slot_same_targets_different_order.sql",
        "fixtures/sql/invalid/direct_param_slot_id_collision.sql",
        "fixtures/sql/invalid/slot_variant_limit_exceeded.sql",
        "fixtures/sql/invalid/slot_variant_invalid_selected_fragment.sql",
        "fixtures/sql/invalid/fragment_param_inference_failure.sql",
        "fixtures/sql/invalid/repeated_slot_fragment_param_type_conflict.sql",
        "fixtures/sql/invalid/slot_variant_row_shape_mismatch.sql",
        "fixtures/sql/invalid/slot_variant_cardinality_mismatch.sql",
    ] {
        assert!(
            repo_path(required_path).exists(),
            "fixture path should exist: {required_path}"
        );
    }

    for legacy_path in [
        "fixtures/mysql/sqlcomp.config.json",
        "fixtures/mysql/queries/type_metadata_matrix.sql",
        "fixtures/sqlcomp/invalid/non_select.sql",
    ] {
        assert!(
            !repo_path(legacy_path).exists(),
            "legacy fixture path should be removed: {legacy_path}"
        );
    }

    assert!(VALID_CONFIG.contains(r#""include": ["valid/**/*.sql"]"#));
    assert!(INVALID_CONFIG.contains(r#""include": ["invalid/**/*.sql"]"#));
}

#[test]
fn fixture_all_column_type_schema_covers_mysql_type_categories_in_order() {
    let schema = INIT_FIXTURES[0];
    let actual_columns = fixture_all_column_type_columns(schema);
    let expected_columns = FIXTURE_ALL_COLUMN_TYPE_COVERAGE
        .iter()
        .flat_map(|column| {
            [
                format!("  {}", column.nullable_definition),
                format!("  {}", column.not_null_definition),
            ]
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual_columns, expected_columns,
        "fixture_all_column_type should list MySQL type categories in coverage order",
    );
}

#[test]
fn fixture_all_column_type_schema_covers_nullable_and_not_null_pairs() {
    let columns = fixture_all_column_type_columns(INIT_FIXTURES[0]);

    for column in FIXTURE_ALL_COLUMN_TYPE_COVERAGE {
        assert!(
            columns
                .iter()
                .any(|schema_column| schema_column.trim() == column.nullable_definition),
            "missing nullable fixture column `{}`",
            column.nullable_definition,
        );
        assert!(
            columns
                .iter()
                .any(|schema_column| schema_column.trim() == column.not_null_definition),
            "missing not-null fixture column `{}`",
            column.not_null_definition,
        );
    }
}

#[test]
fn extracts_sqlcomp_query_bodies() {
    let queries = extract_sqlcomp_queries(
        r"
/* @sqlcomp
{
  type: query
  id: first
}
*/
SELECT 1;

/* @sqlcomp
{
  type: query
  id: second
}
*/
SELECT 2;
",
    )
    .expect("query extraction should pass source intake");

    assert_eq!(queries, vec!["SELECT 1;", "SELECT 2;"]);
}

#[test]
fn extracted_sqlcomp_query_bodies_use_param_analysis_sql() {
    let queries = extract_sqlcomp_queries(
        r"
/* @sqlcomp
{
  type: query
  id: findUser
}
*/
SELECT id
FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: string } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
    )
    .expect("Param query extraction should pass source intake");

    assert_eq!(queries, vec!["SELECT id\nFROM users\nWHERE email = ?;"]);
}
