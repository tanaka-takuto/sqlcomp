//! sqlx-backed `MySQL` metadata adapter.

use sqlcomp_app::MetadataProvider;
use sqlcomp_core as core;
use sqlx::{AssertSqlSafe, Column, Connection, Executor, MySqlConnection, SqlSafeStr, TypeInfo};

/// sqlx-backed `MySQL` metadata provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlxMysqlMetadataProvider {
    database_url: String,
}

impl SqlxMysqlMetadataProvider {
    /// Build a provider for the configured `MySQL` database URL.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }

    /// Configured database URL.
    #[must_use]
    pub fn database_url(&self) -> &str {
        &self.database_url
    }
}

impl MetadataProvider for SqlxMysqlMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        if tokio::runtime::Handle::try_current().is_ok() {
            describe_query_metadata_on_worker_thread(self.database_url().to_owned(), query.clone())
        } else {
            describe_query_metadata_blocking(self.database_url(), query)
        }
    }
}

fn describe_query_metadata_on_worker_thread(
    database_url: String,
    query: core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let error_query = query.clone();
    std::thread::spawn(move || describe_query_metadata_blocking(&database_url, &query))
        .join()
        .unwrap_or_else(|_| {
            Err(query_error(
                &error_query,
                "MySQL metadata worker thread panicked",
            ))
        })
}

fn describe_query_metadata_blocking(
    database_url: &str,
    query: &core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            query_error(
                query,
                format!("failed to create MySQL metadata runtime: {error}"),
            )
        })?;

    runtime.block_on(describe_query_metadata(database_url, query))
}

async fn describe_query_metadata(
    database_url: &str,
    query: &core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let mut connection = MySqlConnection::connect(database_url)
        .await
        .map_err(|error| {
            query_error(
                query,
                format!("failed to connect to MySQL database: {error}"),
            )
        })?;

    // The dialect analyzer has already accepted this query as the MVP's single
    // SELECT statement shape. sqlx requires the assertion for dynamic SQL text.
    let description = connection
        .describe(AssertSqlSafe(query.sql().to_owned()).into_sql_str())
        .await
        .map_err(|error| query_error(query, format!("failed to describe MySQL query: {error}")))?;

    Ok(core::DbQueryMetadata::new(
        description
            .columns()
            .iter()
            .enumerate()
            .map(|(index, column)| {
                map_mysql_result_column_metadata(
                    column.name(),
                    column.type_info().name(),
                    description.nullable(index),
                )
            })
            .collect(),
    ))
}

fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

/// Map one `MySQL` result column description into core metadata.
#[must_use]
pub fn map_mysql_result_column_metadata(
    name: &str,
    type_name: &str,
    nullable: Option<bool>,
) -> core::DbResultColumn {
    core::DbResultColumn::new(
        name.to_owned(),
        mysql_type_name_to_core_type(type_name),
        nullable,
    )
}

fn mysql_type_name_to_core_type(type_name: &str) -> core::CoreType {
    let normalized = normalized_mysql_type_name(type_name);
    let (base_type, is_unsigned) = normalized
        .strip_suffix(" UNSIGNED")
        .map_or((normalized.as_str(), false), |base_type| (base_type, true));

    match base_type {
        "BOOL" | "BOOLEAN" => core::CoreType::Bool,
        "INT" | "INTEGER" if is_unsigned => core::CoreType::Int64,
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" => core::CoreType::Int32,
        "BIGINT" if is_unsigned => core::CoreType::Unknown,
        "BIGINT" => core::CoreType::Int64,
        "DEC" | "DECIMAL" | "FIXED" | "NUMERIC" => core::CoreType::Decimal,
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" | "REAL" => core::CoreType::Float64,
        "CHAR" | "ENUM" | "LONGTEXT" | "MEDIUMTEXT" | "SET" | "TEXT" | "TINYTEXT" | "VARCHAR" => {
            core::CoreType::String
        }
        "BINARY" | "BLOB" | "LONGBLOB" | "MEDIUMBLOB" | "TINYBLOB" | "VARBINARY" => {
            core::CoreType::Bytes
        }
        "DATE" => core::CoreType::Date,
        "TIME" => core::CoreType::Time,
        "DATETIME" | "TIMESTAMP" => core::CoreType::DateTime,
        "JSON" => core::CoreType::Json,
        _ => core::CoreType::Unknown,
    }
}

fn normalized_mysql_type_name(type_name: &str) -> String {
    let mut without_precision = String::with_capacity(type_name.len());
    let mut precision_depth = 0_u8;

    for character in type_name.trim().chars() {
        match character {
            '(' => precision_depth = precision_depth.saturating_add(1),
            ')' if precision_depth > 0 => precision_depth -= 1,
            _ if precision_depth == 0 => without_precision.push(character),
            _ => {}
        }
    }

    let mut collapsed = String::with_capacity(without_precision.len());
    for word in without_precision.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }

    collapsed.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::map_mysql_result_column_metadata;
    use sqlcomp_core as core;

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
}
