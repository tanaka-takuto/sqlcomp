use super::super::{resolve_param_usage_metadata, resolve_result_column_type_refs};
use super::*;

#[test]
fn schema_columns_preserve_mysql_enum_value_type_refs() {
    let column = schema_enum_column("orders", "status", ["draft", "paid"]);

    assert_eq!(column.ty, core::CoreType::String);
    assert_eq!(
        column.type_ref.enum_values(),
        Some(["draft".to_owned(), "paid".to_owned()].as_slice())
    );
    assert_ne!(
        column.type_ref,
        core::CoreTypeRef::from(core::CoreType::String)
    );
}

#[test]
fn resolves_param_type_refs_from_schema_backed_enum_columns() {
    let query = raw_param_query(
        "SELECT o.id FROM orders AS o WHERE o.status = ?;",
        [core::ParamUsage::new(
            "status".to_owned(),
            None,
            false,
            core::SourceLocation::unknown(),
        )],
    );
    let schema_columns = [schema_enum_column("orders", "status", ["draft", "paid"])];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("schema-backed enum Param context should resolve");

    assert_eq!(params[0].ty(), core::CoreType::String);
    assert_eq!(
        params[0].type_ref().enum_values(),
        Some(["draft".to_owned(), "paid".to_owned()].as_slice())
    );
}

#[test]
fn resolves_result_type_refs_from_schema_backed_direct_projection_columns() {
    let query = raw_param_query(
        "SELECT o.status AS orderStatus, o.total_amount FROM orders AS o;",
        Vec::<core::ParamUsage>::new(),
    );
    let schema_columns = [
        schema_enum_column("orders", "status", ["draft", "paid"]),
        schema_column("orders", "total_amount", core::CoreType::Decimal),
    ];

    let result_type_refs = resolve_result_column_type_refs(&query, &schema_columns)
        .expect("schema-backed direct projection columns should resolve");

    assert_eq!(result_type_refs.len(), 2);
    assert_eq!(
        result_type_refs[0]
            .as_ref()
            .and_then(core::CoreTypeRef::enum_values),
        Some(["draft".to_owned(), "paid".to_owned()].as_slice())
    );
    assert_eq!(
        result_type_refs[1],
        Some(core::CoreTypeRef::from(core::CoreType::Decimal))
    );
}

fn schema_enum_column(
    table_name: &str,
    column_name: &str,
    values: impl IntoIterator<Item = &'static str>,
) -> MysqlSchemaColumn {
    let values = values.into_iter().collect::<Vec<_>>();
    let column_type = format!(
        "enum({})",
        values
            .iter()
            .map(|value| format!("'{value}'"))
            .collect::<Vec<_>>()
            .join(",")
    );
    MysqlSchemaColumn::new_current_database(
        table_name.to_owned(),
        column_name.to_owned(),
        column_type,
        core::CoreTypeRef::from_enum_values(values.into_iter().map(str::to_owned).collect())
            .expect("test enum values should build a Core type reference"),
    )
}
