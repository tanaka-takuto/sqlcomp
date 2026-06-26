use ::sqlx::{AssertSqlSafe, MySqlConnection, Row, SqlSafeStr};
use sqlay_core as core;

use super::diagnostics::{mutation_error, query_error};
use super::param_inference::{current_database_mutation_table_names, current_database_table_names};
use super::result_mapping::mysql_type_name_to_core_type;

pub(super) async fn fetch_current_database_schema_columns(
    connection: &mut MySqlConnection,
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let table_names = current_database_table_names(query)?;
    fetch_schema_columns_for_tables(connection, &table_names, |error| {
        query_error(
            query,
            format!("failed to describe MySQL schema columns: {error}"),
        )
    })
    .await
}

pub(super) async fn fetch_current_database_mutation_schema_columns(
    connection: &mut MySqlConnection,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let table_names = current_database_mutation_table_names(mutation)?;
    fetch_schema_columns_for_tables(connection, &table_names, |error| {
        mutation_error(
            mutation,
            format!("failed to describe MySQL schema columns: {error}"),
        )
    })
    .await
}

async fn fetch_schema_columns_for_tables(
    connection: &mut MySqlConnection,
    table_names: &[String],
    on_error: impl Fn(::sqlx::Error) -> core::DiagnosticReport,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    if table_names.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat_n("?", table_names.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT TABLE_NAME AS table_name, COLUMN_NAME AS column_name, COLUMN_TYPE AS column_type \
         FROM information_schema.columns \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN ({placeholders})"
    );
    let mut schema_query = ::sqlx::query(AssertSqlSafe(sql).into_sql_str());
    for table_name in table_names {
        schema_query = schema_query.bind(table_name);
    }

    let rows = schema_query.fetch_all(connection).await.map_err(on_error)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let table_name: String = row.get("table_name");
            let column_name: String = row.get("column_name");
            let column_type: String = row.get("column_type");

            MysqlSchemaColumn::new(
                table_name,
                column_name,
                mysql_type_name_to_core_type(&column_type),
            )
        })
        .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MysqlSchemaColumn {
    pub(super) table_name: String,
    pub(super) column_name: String,
    pub(super) ty: core::CoreType,
}

impl MysqlSchemaColumn {
    pub(super) const fn new(table_name: String, column_name: String, ty: core::CoreType) -> Self {
        Self {
            table_name,
            column_name,
            ty,
        }
    }
}
