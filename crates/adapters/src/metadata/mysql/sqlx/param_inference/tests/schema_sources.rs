use super::super::super::schema_columns::{MysqlSchemaColumn, MysqlSchemaTableRef};
use super::super::{mutation_schema_table_refs, schema_table_refs};
use super::*;

#[test]
fn resolves_param_types_from_schema_qualified_table_sources() {
    let query = raw_param_query(
        "SELECT o.id FROM billing.orders AS o WHERE o.status = ? AND o.total_amount >= ?;",
        [
            param_usage("status", None),
            param_usage("minimumAmount", None),
        ],
    );
    let schema_columns = [
        schema_column_in_database("billing", "orders", "status", core::CoreType::String),
        schema_column_in_database("billing", "orders", "total_amount", core::CoreType::Decimal),
    ];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("explicit database table sources should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("status".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("minimumAmount".to_owned(), core::CoreType::Decimal),
        ]
    );
}

#[test]
fn rejects_ambiguous_bare_select_table_names_without_value_type() {
    let query = raw_param_query(
        "SELECT billing.orders.id FROM billing.orders JOIN archive.orders ON archive.orders.id = billing.orders.id WHERE orders.status = ?;",
        [param_usage("status", None)],
    );
    let schema_columns = [
        schema_column_in_database("billing", "orders", "status", core::CoreType::String),
        schema_column_in_database("archive", "orders", "status", core::CoreType::Bool),
    ];

    let report = resolve_param_usage_metadata(&query, &schema_columns)
        .expect_err("ambiguous bare table names should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "status",
            "table alias `orders` does not resolve to a supported schema-backed table"
        )
    );
}

#[test]
fn schema_table_refs_include_current_and_explicit_database_sources() {
    let query = raw_param_query(
        "SELECT u.id FROM users AS u JOIN billing.orders AS o ON o.user_id = u.id WHERE u.email = ? AND o.status = ?;",
        [param_usage("email", None), param_usage("status", None)],
    );

    let refs = schema_table_refs(&query).expect("query table refs should parse");

    assert_eq!(
        refs,
        [
            MysqlSchemaTableRef::current_database("users"),
            MysqlSchemaTableRef::explicit_database("billing", "orders"),
        ]
    );
}

#[test]
fn mysql_schema_columns_preserve_source_identity_and_column_type() {
    let column = MysqlSchemaColumn::new_explicit_database(
        "billing".to_owned(),
        "orders".to_owned(),
        "status".to_owned(),
        "enum('draft','paid')".to_owned(),
        core::CoreType::String,
    );

    assert_eq!(
        column.table_ref,
        MysqlSchemaTableRef::explicit_database("billing", "orders")
    );
    assert_eq!(column.column_name, "status");
    assert_eq!(column.column_type, "enum('draft','paid')");
    assert_eq!(column.ty, core::CoreType::String);
}

#[test]
fn rejects_table_identifiers_containing_dots_without_value_type() {
    let query = raw_param_query(
        "SELECT u.id FROM `app.users` AS u WHERE u.email = ?;",
        [core::ParamUsage::new(
            "email".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );

    let report = resolve_param_usage_metadata(&query, &[])
        .expect_err("table identifiers containing dots should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "email",
            "table alias `u` does not resolve to a supported schema-backed table"
        )
    );
}

#[test]
fn mutation_schema_table_refs_include_subquery_tables_used_by_params() {
    let mutation = raw_param_mutation(
        "UPDATE users AS u SET u.name = ? WHERE EXISTS (SELECT 1 FROM accounts AS a WHERE a.user_id = u.id AND a.status = ?) AND u.email = ?;",
        [
            param_usage("name", None),
            param_usage("accountStatus", None),
            param_usage("email", None),
        ],
    );

    let refs = mutation_schema_table_refs(&mutation)
        .expect("mutation schema lookup should include subquery tables used by Params");

    assert_eq!(
        refs,
        [
            MysqlSchemaTableRef::current_database("accounts"),
            MysqlSchemaTableRef::current_database("users"),
        ]
    );
}

#[test]
fn resolves_schema_qualified_mutation_target_param_types() {
    let mutation = raw_param_mutation(
        "UPDATE billing.orders AS o SET o.status = ? WHERE o.id = ?;",
        [param_usage("status", None), param_usage("orderId", None)],
    );
    let schema_columns = [
        schema_column_in_database("billing", "orders", "id", core::CoreType::Int64),
        schema_column_in_database("billing", "orders", "status", core::CoreType::String),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("explicit database mutation targets should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("status".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("orderId".to_owned(), core::CoreType::Int64),
        ]
    );
}

#[test]
fn resolves_current_database_three_part_mutation_param_types() {
    let mutation = raw_param_mutation(
        "UPDATE orders SET sqlay.orders.status = ? WHERE sqlay.orders.id = ?;",
        [param_usage("status", None), param_usage("orderId", None)],
    );
    let schema_columns = [
        schema_column("orders", "id", core::CoreType::Int64),
        schema_column("orders", "status", core::CoreType::String),
        schema_column_in_database("sqlay", "orders", "id", core::CoreType::Int64),
        schema_column_in_database("sqlay", "orders", "status", core::CoreType::String),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("current-database three-part mutation contexts should resolve");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("status".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("orderId".to_owned(), core::CoreType::Int64),
        ]
    );
}

#[test]
fn rejects_ambiguous_bare_mutation_table_names_without_value_type() {
    let mutation = raw_param_mutation(
        "UPDATE billing.orders JOIN archive.orders ON archive.orders.id = billing.orders.id SET billing.orders.status = ? WHERE orders.id = ?;",
        [param_usage("status", None), param_usage("orderId", None)],
    );
    let schema_columns = [
        schema_column_in_database("billing", "orders", "status", core::CoreType::String),
        schema_column_in_database("archive", "orders", "id", core::CoreType::Int64),
    ];

    let report = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect_err("ambiguous bare mutation table names should require valueType");

    assert_eq!(
        report.diagnostics()[0].message(),
        param_value_type_required_message(
            "orderId",
            "table alias `orders` does not resolve to a supported schema-backed table"
        )
    );
}
