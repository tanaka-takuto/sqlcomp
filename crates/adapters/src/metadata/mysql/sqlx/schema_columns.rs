use ::sqlx::{AssertSqlSafe, MySqlConnection, Row, SqlSafeStr};
use sqlay_core as core;

use super::diagnostics::{mutation_error, query_error};
use super::param_inference::{mutation_schema_table_refs, schema_table_refs};
use super::result_mapping::mysql_type_name_to_core_type;

pub(super) async fn fetch_schema_columns(
    connection: &mut MySqlConnection,
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let table_refs = schema_table_refs(query)?;
    fetch_schema_columns_for_table_refs(connection, &table_refs, |error| {
        query_error(
            query,
            format!("failed to describe MySQL schema columns: {error}"),
        )
    })
    .await
}

pub(super) async fn fetch_mutation_schema_columns(
    connection: &mut MySqlConnection,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let table_refs = mutation_schema_table_refs(mutation)?;
    fetch_schema_columns_for_table_refs(connection, &table_refs, |error| {
        mutation_error(
            mutation,
            format!("failed to describe MySQL schema columns: {error}"),
        )
    })
    .await
}

async fn fetch_schema_columns_for_table_refs(
    connection: &mut MySqlConnection,
    table_refs: &[MysqlSchemaTableRef],
    on_error: impl Fn(::sqlx::Error) -> core::DiagnosticReport + Send + Sync,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    if table_refs.is_empty() {
        return Ok(Vec::new());
    }

    let mut columns = Vec::new();
    columns.extend(fetch_current_database_schema_columns(connection, table_refs, &on_error).await?);
    columns.extend(fetch_explicit_database_schema_columns(connection, table_refs, on_error).await?);

    Ok(columns)
}

async fn fetch_current_database_schema_columns(
    connection: &mut MySqlConnection,
    table_refs: &[MysqlSchemaTableRef],
    on_error: &(impl Fn(::sqlx::Error) -> core::DiagnosticReport + Send + Sync),
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let table_names = table_refs
        .iter()
        .filter_map(|table_ref| match table_ref {
            MysqlSchemaTableRef::CurrentDatabase { table_name } => Some(table_name.as_str()),
            MysqlSchemaTableRef::ExplicitDatabase { .. } => None,
        })
        .collect::<Vec<_>>();
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

            current_database_schema_column(table_name, column_name, column_type)
        })
        .collect())
}

async fn fetch_explicit_database_schema_columns(
    connection: &mut MySqlConnection,
    table_refs: &[MysqlSchemaTableRef],
    on_error: impl Fn(::sqlx::Error) -> core::DiagnosticReport + Send + Sync,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let explicit_refs = table_refs
        .iter()
        .filter_map(|table_ref| match table_ref {
            MysqlSchemaTableRef::CurrentDatabase { .. } => None,
            MysqlSchemaTableRef::ExplicitDatabase {
                database_name,
                table_name,
            } => Some((database_name.as_str(), table_name.as_str())),
        })
        .collect::<Vec<_>>();
    if explicit_refs.is_empty() {
        return Ok(Vec::new());
    }

    let conditions =
        std::iter::repeat_n("(TABLE_SCHEMA = ? AND TABLE_NAME = ?)", explicit_refs.len())
            .collect::<Vec<_>>()
            .join(" OR ");
    let sql = format!(
        "SELECT TABLE_SCHEMA AS database_name, TABLE_NAME AS table_name, COLUMN_NAME AS column_name, COLUMN_TYPE AS column_type \
         FROM information_schema.columns \
         WHERE {conditions}"
    );
    let mut schema_query = ::sqlx::query(AssertSqlSafe(sql).into_sql_str());
    for (database_name, table_name) in explicit_refs {
        schema_query = schema_query.bind(database_name).bind(table_name);
    }

    let rows = schema_query.fetch_all(connection).await.map_err(on_error)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let database_name: String = row.get("database_name");
            let table_name: String = row.get("table_name");
            let column_name: String = row.get("column_name");
            let column_type: String = row.get("column_type");

            explicit_database_schema_column(database_name, table_name, column_name, column_type)
        })
        .collect())
}

fn current_database_schema_column(
    table_name: String,
    column_name: String,
    column_type: String,
) -> MysqlSchemaColumn {
    let ty = mysql_type_name_to_core_type(&column_type);
    MysqlSchemaColumn::new_current_database(table_name, column_name, column_type, ty)
}

fn explicit_database_schema_column(
    database_name: String,
    table_name: String,
    column_name: String,
    column_type: String,
) -> MysqlSchemaColumn {
    let ty = mysql_type_name_to_core_type(&column_type);
    MysqlSchemaColumn::new_explicit_database(
        database_name,
        table_name,
        column_name,
        column_type,
        ty,
    )
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum MysqlSchemaTableRef {
    CurrentDatabase {
        table_name: String,
    },
    ExplicitDatabase {
        database_name: String,
        table_name: String,
    },
}

impl MysqlSchemaTableRef {
    pub(super) fn current_database(table_name: impl Into<String>) -> Self {
        Self::CurrentDatabase {
            table_name: table_name.into(),
        }
    }

    pub(super) fn explicit_database(
        database_name: impl Into<String>,
        table_name: impl Into<String>,
    ) -> Self {
        Self::ExplicitDatabase {
            database_name: database_name.into(),
            table_name: table_name.into(),
        }
    }

    pub(super) fn table_name(&self) -> &str {
        match self {
            Self::CurrentDatabase { table_name } | Self::ExplicitDatabase { table_name, .. } => {
                table_name
            }
        }
    }

    pub(super) fn qualifier_key(&self) -> Option<String> {
        match self {
            Self::CurrentDatabase { .. } => None,
            Self::ExplicitDatabase {
                database_name,
                table_name,
            } => Some(format!("{database_name}.{table_name}")),
        }
    }

    pub(super) fn table_description(&self) -> String {
        match self {
            Self::CurrentDatabase { table_name } => {
                format!("current-database table `{table_name}`")
            }
            Self::ExplicitDatabase {
                database_name,
                table_name,
            } => format!("schema-qualified table `{database_name}.{table_name}`"),
        }
    }

    pub(super) fn column_description(&self, column_name: &str) -> String {
        match self {
            Self::CurrentDatabase { table_name } => format!("{table_name}.{column_name}"),
            Self::ExplicitDatabase {
                database_name,
                table_name,
            } => format!("{database_name}.{table_name}.{column_name}"),
        }
    }

    pub(super) fn unknown_column_message(&self, param_id: &str, column_name: &str) -> String {
        match self {
            Self::CurrentDatabase { .. } => format!(
                "Param `{param_id}` references unknown current-database column `{}`",
                self.column_description(column_name)
            ),
            Self::ExplicitDatabase { .. } => format!(
                "Param `{param_id}` references unknown schema-backed column `{}`",
                self.column_description(column_name)
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MysqlSchemaColumn {
    pub(super) table_ref: MysqlSchemaTableRef,
    pub(super) column_name: String,
    pub(super) column_type: String,
    pub(super) ty: core::CoreType,
}

impl MysqlSchemaColumn {
    pub(super) fn new_current_database(
        table_name: String,
        column_name: String,
        column_type: String,
        ty: core::CoreType,
    ) -> Self {
        Self {
            table_ref: MysqlSchemaTableRef::current_database(table_name),
            column_name,
            column_type,
            ty,
        }
    }

    pub(super) fn new_explicit_database(
        database_name: String,
        table_name: String,
        column_name: String,
        column_type: String,
        ty: core::CoreType,
    ) -> Self {
        Self {
            table_ref: MysqlSchemaTableRef::explicit_database(database_name, table_name),
            column_name,
            column_type,
            ty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_database_schema_column_uses_column_type_details_for_core_type() {
        let column = current_database_schema_column(
            "orders".to_owned(),
            "quantity".to_owned(),
            "int unsigned".to_owned(),
        );

        assert_eq!(
            column.table_ref,
            MysqlSchemaTableRef::current_database("orders")
        );
        assert_eq!(column.column_name, "quantity");
        assert_eq!(column.column_type, "int unsigned");
        assert_eq!(column.ty, core::CoreType::Int64);
    }

    #[test]
    fn explicit_database_schema_column_uses_column_type_details_for_core_type() {
        let column = explicit_database_schema_column(
            "billing".to_owned(),
            "orders".to_owned(),
            "quantity".to_owned(),
            "bigint unsigned".to_owned(),
        );

        assert_eq!(
            column.table_ref,
            MysqlSchemaTableRef::explicit_database("billing", "orders")
        );
        assert_eq!(column.column_name, "quantity");
        assert_eq!(column.column_type, "bigint unsigned");
        assert_eq!(column.ty, core::CoreType::Unknown);
    }
}
