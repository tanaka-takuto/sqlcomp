use ::sqlx::{AssertSqlSafe, Column, Connection, Executor, MySqlConnection, SqlSafeStr, TypeInfo};
use sqlay_app::{MetadataProvider, MutationMetadataProvider};
use sqlay_core as core;

use super::diagnostics::{mutation_error, query_error};
use super::param_inference::{resolve_mutation_param_usage_metadata, resolve_param_usage_metadata};
use super::result_mapping::map_mysql_result_column_metadata;
use super::schema_columns::{
    fetch_current_database_mutation_schema_columns, fetch_current_database_schema_columns,
};

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

impl MutationMetadataProvider for SqlxMysqlMetadataProvider {
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        _analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata> {
        if tokio::runtime::Handle::try_current().is_ok() {
            describe_mutation_metadata_on_worker_thread(
                self.database_url().to_owned(),
                mutation.clone(),
            )
        } else {
            describe_mutation_metadata_blocking(self.database_url(), mutation)
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

fn describe_mutation_metadata_on_worker_thread(
    database_url: String,
    mutation: core::RawMutation,
) -> core::DiagnosticResult<core::DbMutationMetadata> {
    let error_mutation = mutation.clone();
    std::thread::spawn(move || describe_mutation_metadata_blocking(&database_url, &mutation))
        .join()
        .unwrap_or_else(|_| {
            Err(mutation_error(
                &error_mutation,
                "MySQL mutation metadata worker thread panicked",
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

fn describe_mutation_metadata_blocking(
    database_url: &str,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<core::DbMutationMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            mutation_error(
                mutation,
                format!("failed to create MySQL metadata runtime: {error}"),
            )
        })?;

    runtime.block_on(describe_mutation_metadata(database_url, mutation))
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

    let param_usages = describe_param_usages(&mut connection, query).await?;

    // The dialect analyzer has already accepted this query as the supported
    // single-SELECT statement shape. sqlx requires the assertion for dynamic SQL
    // text.
    let description = connection
        .describe(AssertSqlSafe(query.analysis_sql().to_owned()).into_sql_str())
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
    )
    .with_param_usages(param_usages))
}

async fn describe_mutation_metadata(
    database_url: &str,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<core::DbMutationMetadata> {
    let mut connection = MySqlConnection::connect(database_url)
        .await
        .map_err(|error| {
            mutation_error(
                mutation,
                format!("failed to connect to MySQL database: {error}"),
            )
        })?;

    let param_usages = describe_mutation_param_usages(&mut connection, mutation).await?;

    Ok(core::DbMutationMetadata::new().with_param_usages(param_usages))
}

async fn describe_param_usages(
    connection: &mut MySqlConnection,
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if query.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let schema_columns = fetch_current_database_schema_columns(connection, query).await?;
    resolve_param_usage_metadata(query, &schema_columns)
}

async fn describe_mutation_param_usages(
    connection: &mut MySqlConnection,
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if mutation.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let schema_columns =
        fetch_current_database_mutation_schema_columns(connection, mutation).await?;
    resolve_mutation_param_usage_metadata(mutation, &schema_columns)
}
