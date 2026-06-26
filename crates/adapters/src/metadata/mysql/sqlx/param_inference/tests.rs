use sqlay_core as core;

use super::super::result_mapping::map_mysql_result_column_metadata;
use super::super::schema_columns::MysqlSchemaColumn;
use super::{
    current_database_mutation_table_names, param_value_type_required_message,
    resolve_mutation_param_usage_metadata, resolve_param_usage_metadata,
};

mod context_traversal;

#[test]
fn maps_representative_mysql_type_names_to_core_types() {
    let cases = [
        ("BOOLEAN", core::CoreType::Bool),
        ("TINYINT", core::CoreType::Int32),
        ("SMALLINT", core::CoreType::Int32),
        ("MEDIUMINT", core::CoreType::Int32),
        ("INT", core::CoreType::Int32),
        ("INTEGER", core::CoreType::Int32),
        ("BIGINT", core::CoreType::Int64),
        ("DECIMAL", core::CoreType::Decimal),
        ("NUMERIC", core::CoreType::Decimal),
        ("FLOAT", core::CoreType::Float64),
        ("DOUBLE", core::CoreType::Float64),
        ("REAL", core::CoreType::Float64),
        ("CHAR", core::CoreType::String),
        ("VARCHAR", core::CoreType::String),
        ("TEXT", core::CoreType::String),
        ("TINYTEXT", core::CoreType::String),
        ("MEDIUMTEXT", core::CoreType::String),
        ("LONGTEXT", core::CoreType::String),
        ("ENUM", core::CoreType::String),
        ("SET", core::CoreType::String),
        ("BINARY", core::CoreType::Bytes),
        ("VARBINARY", core::CoreType::Bytes),
        ("BLOB", core::CoreType::Bytes),
        ("TINYBLOB", core::CoreType::Bytes),
        ("MEDIUMBLOB", core::CoreType::Bytes),
        ("LONGBLOB", core::CoreType::Bytes),
        ("DATE", core::CoreType::Date),
        ("TIME", core::CoreType::Time),
        ("DATETIME", core::CoreType::DateTime),
        ("TIMESTAMP", core::CoreType::DateTime),
        ("JSON", core::CoreType::Json),
    ];

    for (type_name, expected_type) in cases {
        let column = map_mysql_result_column_metadata("value", type_name, Some(false));

        assert_eq!(
            column,
            core::DbResultColumn::new("value".to_owned(), expected_type, Some(false)),
            "{type_name} should map to {expected_type:?}"
        );
    }
}

#[test]
fn maps_unknown_mysql_type_names_conservatively() {
    let column = map_mysql_result_column_metadata("shape", "GEOMETRY", Some(false));

    assert_eq!(
        column,
        core::DbResultColumn::new("shape".to_owned(), core::CoreType::Unknown, Some(false))
    );
}

#[test]
fn preserves_mysql_nullability_metadata_for_core_ir() {
    let nullable = map_mysql_result_column_metadata("nickname", "VARCHAR", Some(true));
    let non_nullable = map_mysql_result_column_metadata("displayName", "VARCHAR", Some(false));

    assert_eq!(
        nullable,
        core::DbResultColumn::new("nickname".to_owned(), core::CoreType::String, Some(true))
    );
    assert!(nullable.to_result_column().is_nullable());

    assert_eq!(
        non_nullable,
        core::DbResultColumn::new(
            "displayName".to_owned(),
            core::CoreType::String,
            Some(false),
        )
    );
    assert!(!non_nullable.to_result_column().is_nullable());
}

#[test]
fn preserves_unknown_nullability_for_core_ir() {
    let column = map_mysql_result_column_metadata("name", "VARCHAR", None);

    assert_eq!(
        column,
        core::DbResultColumn::new("name".to_owned(), core::CoreType::String, None)
    );
    assert!(column.to_result_column().is_nullable());
}

#[test]
fn normalizes_case_and_precision_suffixes() {
    let column = map_mysql_result_column_metadata("amount", "decimal(18, 4)", Some(false));

    assert_eq!(
        column,
        core::DbResultColumn::new("amount".to_owned(), core::CoreType::Decimal, Some(false))
    );

    let widened = map_mysql_result_column_metadata("count", "int(10) unsigned", Some(false));

    assert_eq!(
        widened,
        core::DbResultColumn::new("count".to_owned(), core::CoreType::Int64, Some(false))
    );

    let unknown = map_mysql_result_column_metadata("id", "BIGINT UNSIGNED", Some(false));

    assert_eq!(
        unknown,
        core::DbResultColumn::new("id".to_owned(), core::CoreType::Unknown, Some(false))
    );
}

#[test]
fn resolves_param_types_from_direct_qualified_column_contexts() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u JOIN accounts ON accounts.user_id = u.id WHERE u.email = ? AND accounts.id <> ? AND accounts.balance >= ? AND u.id IN (?, ?);",
        [
            core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "accountId".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "minimumBalance".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "primaryUserId".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "secondaryUserId".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [
        schema_column("users", "id", core::CoreType::Int64),
        schema_column("users", "email", core::CoreType::String),
        schema_column("accounts", "id", core::CoreType::Int64),
        schema_column("accounts", "balance", core::CoreType::Decimal),
    ];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("qualified direct column contexts should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("accountId".to_owned(), core::CoreType::Int64),
            core::DbParamUsage::new("minimumBalance".to_owned(), core::CoreType::Decimal),
            core::DbParamUsage::new("primaryUserId".to_owned(), core::CoreType::Int64),
            core::DbParamUsage::new("secondaryUserId".to_owned(), core::CoreType::Int64),
        ]
    );
}

#[test]
fn value_type_override_skips_direct_column_inference() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u WHERE unknown_alias.email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            Some(core::CoreType::String),
            false,
            core::SourceLocation::unknown(),
        )],
    );

    let params = resolve_param_usage_metadata(&query, &[])
        .expect("valueType override should not require schema inference");

    assert_eq!(
        params,
        [core::DbParamUsage::new(
            "email".to_owned(),
            core::CoreType::String
        )]
    );
}

#[test]
fn value_type_override_allows_params_in_untraversed_query_clauses() {
    let query = raw_param_query(
        "SELECT COUNT(*) FROM users AS u GROUP BY ? ORDER BY ? LIMIT ?;",
        [
            core::ParamUsage::new(
                "groupKey".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "sortKey".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "limitCount".to_owned(),
                Some(core::CoreType::Int32),
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [schema_column("users", "id", core::CoreType::Int64)];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("valueType should allow unsupported query clause Param contexts");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("groupKey".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("sortKey".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("limitCount".to_owned(), core::CoreType::Int32),
        ]
    );
}

#[test]
fn value_type_override_in_subquery_preserves_later_inference_order() {
    let query = raw_param_query(
        "SELECT (SELECT ?) AS marker FROM users AS u WHERE u.email = ?;",
        [
            core::ParamUsage::new(
                "marker".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("subquery valueType should not shift later Param inference");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("marker".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        ]
    );
}

#[test]
fn value_type_override_in_json_table_preserves_later_inference_order() {
    let query = raw_param_query(
        "SELECT u.id FROM JSON_TABLE(?, '$[*]' COLUMNS (id BIGINT PATH '$.id')) AS jt JOIN users AS u ON u.id = jt.id WHERE u.email = ?;",
        [
            core::ParamUsage::new(
                "jsonRows".to_owned(),
                Some(core::CoreType::Json),
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [
        schema_column("users", "id", core::CoreType::Int64),
        schema_column("users", "email", core::CoreType::String),
    ];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("JSON_TABLE valueType should not shift later Param inference");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("jsonRows".to_owned(), core::CoreType::Json),
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        ]
    );
}

#[test]
fn rejects_param_without_value_type_when_context_is_not_supported() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u WHERE COALESCE(?, u.email) = u.email;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("function context should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "email",
            "no supported qualified column context was found"
        )
    );
}

#[test]
fn rejects_nullable_filter_param_without_value_type_with_actionable_guidance() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u WHERE ? IS NULL;",
        [core::ParamUsage::new(
            "emailFilter".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("NULL sample optional-filter context should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        "Param `emailFilter` requires `valueType` because no supported qualified column context was found; use an inline `valueType` such as `valueType: string` or compare the Param directly with a qualified column; supported values are `bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`; use `nullable: true` for nullable inputs"
    );
}

#[test]
fn rejects_unqualified_column_inference_without_value_type() {
    let query = raw_param_query(
        "SELECT id FROM users WHERE email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("unqualified columns should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "email",
            "no supported qualified column context was found"
        )
    );
}

#[test]
fn rejects_schema_qualified_table_inference_without_value_type() {
    let query = raw_param_query(
        "SELECT u.id FROM app.users AS u WHERE u.email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );

    let report = resolve_param_usage_metadata(&query, &[])
        .expect_err("schema-qualified tables should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "email",
            "table alias `u` does not resolve to a current-database table"
        )
    );
}

#[test]
fn rejects_cte_source_shadowing_real_table_without_value_type() {
    let query = raw_param_query(
        "WITH u AS (SELECT id FROM users) SELECT u.id FROM u WHERE u.id = ?;",
        [core::ParamUsage::new(
            "userId".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("u", "id", core::CoreType::Int64)];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("CTE names should shadow current-database tables");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "userId",
            "table alias `u` does not resolve to a current-database table"
        )
    );
}

#[test]
fn resolves_table_aliases_inside_nested_joins() {
    let query = raw_param_query(
        "SELECT u.id FROM (users AS u JOIN accounts AS a ON a.user_id = u.id) WHERE u.email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("aliases inside parenthesized joins should resolve");

    assert_eq!(
        params,
        [core::DbParamUsage::new(
            "email".to_owned(),
            core::CoreType::String
        )]
    );
}

#[test]
fn rejects_unknown_alias_without_value_type() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u WHERE missing.email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("unknown table aliases should be diagnosed");

    assert_eq!(
        report.diagnostics()[0].message(),
        "Param `email` references unknown table alias `missing`"
    );
}

#[test]
fn rejects_unknown_current_database_column_without_value_type() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u WHERE u.missing_email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "id", core::CoreType::Int64)];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("unknown current-database columns should be diagnosed");

    assert_eq!(
        report.diagnostics()[0].message(),
        "Param `email` references unknown current-database column `users.missing_email`"
    );
}

#[test]
fn resolves_insert_values_and_on_duplicate_update_mutation_param_types() {
    let mutation = raw_param_mutation(
        "INSERT INTO users (email, name) VALUES (?, ?) ON DUPLICATE KEY UPDATE name = ?, updated_at = ?;",
        [
            core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "name".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "upsertName".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "updatedAt".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [
        schema_column("users", "email", core::CoreType::String),
        schema_column("users", "name", core::CoreType::String),
        schema_column("users", "updated_at", core::CoreType::DateTime),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("direct mutation column contexts should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("name".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("upsertName".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("updatedAt".to_owned(), core::CoreType::DateTime),
        ]
    );
}

#[test]
fn resolves_update_set_qualified_predicate_and_in_mutation_param_types() {
    let mutation = raw_param_mutation(
        "UPDATE users AS u SET u.name = ? WHERE u.id IN (?, ?) AND u.email = ?;",
        [
            core::ParamUsage::new(
                "name".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "primaryUserId".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "secondaryUserId".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [
        schema_column("users", "id", core::CoreType::Int64),
        schema_column("users", "name", core::CoreType::String),
        schema_column("users", "email", core::CoreType::String),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("direct update column contexts should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("name".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("primaryUserId".to_owned(), core::CoreType::Int64),
            core::DbParamUsage::new("secondaryUserId".to_owned(), core::CoreType::Int64),
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        ]
    );
}

#[test]
fn resolves_mutation_subquery_param_types_from_select_direct_column_contexts() {
    let mutation = raw_param_mutation(
        "UPDATE users AS u SET u.name = ? WHERE EXISTS (SELECT 1 FROM accounts AS a WHERE a.user_id = u.id AND a.status = ?) AND u.email = ?;",
        [
            core::ParamUsage::new(
                "name".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "accountStatus".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
            core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            ),
        ],
    );
    let schema_columns = [
        schema_column("users", "name", core::CoreType::String),
        schema_column("users", "email", core::CoreType::String),
        schema_column("accounts", "status", core::CoreType::Bool),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("subquery SELECT direct column contexts should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("name".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("accountStatus".to_owned(), core::CoreType::Bool),
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        ]
    );
}

#[test]
fn mutation_table_names_include_subquery_tables_used_by_params() {
    let mutation = raw_param_mutation(
        "UPDATE users AS u SET u.name = ? WHERE EXISTS (SELECT 1 FROM accounts AS a WHERE a.user_id = u.id AND a.status = ?) AND u.email = ?;",
        [
            param_usage("name", None),
            param_usage("accountStatus", None),
            param_usage("email", None),
        ],
    );

    let table_names = current_database_mutation_table_names(&mutation)
        .expect("mutation schema lookup should include subquery tables used by Params");

    assert_eq!(table_names, ["accounts".to_owned(), "users".to_owned()]);
}

#[test]
fn resolves_insert_set_replace_set_and_delete_mutation_param_types() {
    let cases = [
        (
            raw_param_mutation(
                "INSERT INTO users SET email = ?, name = ?;",
                [param_usage("email", None), param_usage("name", None)],
            ),
            vec![
                core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("name".to_owned(), core::CoreType::String),
            ],
        ),
        (
            raw_param_mutation(
                "REPLACE INTO users SET id = ?, email = ?;",
                [param_usage("id", None), param_usage("email", None)],
            ),
            vec![
                core::DbParamUsage::new("id".to_owned(), core::CoreType::Int64),
                core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
            ],
        ),
        (
            raw_param_mutation(
                "DELETE FROM users AS u WHERE ? = u.email AND u.id IN (?, ?) ORDER BY ? LIMIT ?;",
                [
                    param_usage("email", None),
                    param_usage("primaryUserId", None),
                    param_usage("secondaryUserId", None),
                    param_usage("sortKey", Some(core::CoreType::String)),
                    param_usage("limitCount", Some(core::CoreType::Int32)),
                ],
            ),
            vec![
                core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("primaryUserId".to_owned(), core::CoreType::Int64),
                core::DbParamUsage::new("secondaryUserId".to_owned(), core::CoreType::Int64),
                core::DbParamUsage::new("sortKey".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("limitCount".to_owned(), core::CoreType::Int32),
            ],
        ),
    ];
    let schema_columns = [
        schema_column("users", "id", core::CoreType::Int64),
        schema_column("users", "email", core::CoreType::String),
        schema_column("users", "name", core::CoreType::String),
    ];

    for (mutation, expected_params) in cases {
        let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
            .unwrap_or_else(|report| panic!("{}", report.diagnostics()[0].message()));

        assert_eq!(params, expected_params);
    }
}

#[test]
fn value_type_override_in_mutation_subquery_preserves_later_inference_order() {
    let mutation = raw_param_mutation(
        "UPDATE users AS u SET u.name = ? WHERE EXISTS (SELECT ? FROM accounts AS a WHERE a.user_id = u.id AND a.status = ?) AND u.email = ?;",
        [
            param_usage("name", None),
            param_usage("subqueryMarker", Some(core::CoreType::String)),
            param_usage("accountStatus", None),
            param_usage("email", None),
        ],
    );
    let schema_columns = [
        schema_column("users", "name", core::CoreType::String),
        schema_column("users", "email", core::CoreType::String),
        schema_column("accounts", "status", core::CoreType::Bool),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("subquery valueType should not shift later Param inference");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("name".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("subqueryMarker".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("accountStatus".to_owned(), core::CoreType::Bool),
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
        ]
    );
}

#[test]
fn rejects_schema_qualified_and_unknown_mutation_columns_without_value_type() {
    let schema_qualified = raw_param_mutation(
        "UPDATE app.users AS u SET u.name = ? WHERE u.id = ?;",
        [param_usage("name", None), param_usage("userId", None)],
    );

    let report = resolve_mutation_param_usage_metadata(&schema_qualified, &[])
        .expect_err("schema-qualified mutation targets should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "name",
            "table alias `u` does not resolve to a current-database table"
        )
    );

    let unknown_column = raw_param_mutation(
        "UPDATE users AS u SET u.missing_name = ? WHERE u.id = ?;",
        [param_usage("name", None), param_usage("userId", None)],
    );
    let schema_columns = [schema_column("users", "id", core::CoreType::Int64)];

    let report = resolve_mutation_param_usage_metadata(&unknown_column, &schema_columns)
        .expect_err("unknown mutation columns should be diagnosed");

    assert_eq!(
        report.diagnostics()[0].message(),
        "Param `name` references unknown current-database column `users.missing_name`"
    );
}

#[test]
fn rejects_column_list_free_insert_mutation_param_without_value_type() {
    let mutation = raw_param_mutation(
        "INSERT INTO users VALUES (?);",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let report = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect_err("column-list-free INSERT Params should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "email",
            "no supported mutation column context was found"
        )
    );
}

#[test]
fn rejects_unqualified_mutation_predicate_param_without_value_type() {
    let mutation = raw_param_mutation(
        "UPDATE users AS u SET u.name = ? WHERE id = ?;",
        [
            core::ParamUsage::new(
                "name".to_owned(),
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
        ],
    );
    let schema_columns = [
        schema_column("users", "id", core::CoreType::Int64),
        schema_column("users", "name", core::CoreType::String),
    ];

    let report = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect_err("unqualified predicate columns should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "userId",
            "no supported mutation column context was found"
        )
    );
}

fn raw_param_query(
    analysis_sql: &str,
    param_usages: impl IntoIterator<Item = core::ParamUsage>,
) -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("findUsers".to_owned(), None),
        analysis_sql.to_owned(),
    )
    .with_analysis_sql(analysis_sql.to_owned())
    .with_param_usages(param_usages.into_iter().collect())
}

fn raw_param_mutation(
    analysis_sql: &str,
    param_usages: impl IntoIterator<Item = core::ParamUsage>,
) -> core::RawMutation {
    core::RawMutation::new(
        core::MutationMetadata::new("writeUsers".to_owned()),
        analysis_sql.to_owned(),
    )
    .with_analysis_sql(analysis_sql.to_owned())
    .with_param_usages(param_usages.into_iter().collect())
}

fn schema_column(table_name: &str, column_name: &str, ty: core::CoreType) -> MysqlSchemaColumn {
    MysqlSchemaColumn::new(table_name.to_owned(), column_name.to_owned(), ty)
}

fn param_usage(id: &str, value_type: Option<core::CoreType>) -> core::ParamUsage {
    core::ParamUsage::new(
        id.to_owned(),
        value_type,
        false,
        core::SourceLocation::unknown(),
    )
}
