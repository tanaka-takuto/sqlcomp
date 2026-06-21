use sqlay_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlay_adapters::metadata::mysql::sqlx::{
    SqlxMysqlMetadataProvider, map_mysql_result_column_metadata,
};
use sqlay_adapters::source_fs::split_sqlay_query_blocks;
use sqlay_app::{DialectAnalyzer, MetadataProvider};
use sqlay_core as core;
use sqlx::{AssertSqlSafe, Column, Connection, Executor, MySqlConnection, SqlSafeStr, TypeInfo};

use super::fixture_support::{
    DATABASE_URL_ENV, INIT_FIXTURES, MYSQL_FIXTURE_LOCK, QUERY_FIXTURES,
    execute_fixture_statements, extract_sqlay_queries, raw_query,
};
use super::type_coverage::{assert_fixture_core_type_matrix, assert_fixture_nullability_matrix};

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn sqlx_mysql_metadata_provider_returns_fixture_query_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let provider = SqlxMysqlMetadataProvider::new(database_url);
    let analyzer = MysqlDialectAnalyzer;
    let mut query_count = 0;
    let mut mapped_columns = Vec::new();

    for fixture in QUERY_FIXTURES {
        for query in split_sqlay_query_blocks(fixture)? {
            query_count += 1;

            let analysis = analyzer.analyze(&query)?;
            let metadata = provider.describe(&query, &analysis)?;

            assert!(
                !metadata.columns().is_empty(),
                "provider should expose columns for query `{}`",
                query.metadata().id()
            );
            mapped_columns.extend(metadata.columns().iter().cloned());
        }
    }

    assert!(
        query_count > 0,
        "query fixtures should contain @sqlay blocks"
    );
    assert_fixture_core_type_matrix(&mapped_columns);
    assert_fixture_nullability_matrix(&mapped_columns);

    Ok(())
}

#[test]
fn sqlx_mysql_metadata_provider_reports_connection_failures_as_diagnostics() {
    let provider = SqlxMysqlMetadataProvider::new("not-a-mysql-url");
    let query = raw_query("SELECT 1 AS value;");
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);

    let report = provider
        .describe(&query, &analysis)
        .expect_err("invalid database URL should fail before metadata lookup");

    assert!(
        report.diagnostics()[0]
            .message()
            .starts_with("failed to connect to MySQL database:"),
        "{}",
        report.diagnostics()[0].message()
    );
}

#[tokio::test]
async fn sqlx_mysql_metadata_provider_reports_connection_failures_inside_tokio_runtime() {
    let provider = SqlxMysqlMetadataProvider::new("not-a-mysql-url");
    let query = raw_query("SELECT 1 AS value;");
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);

    let report = provider
        .describe(&query, &analysis)
        .expect_err("invalid database URL should fail without panicking inside Tokio");

    assert!(
        report.diagnostics()[0]
            .message()
            .starts_with("failed to connect to MySQL database:"),
        "{}",
        report.diagnostics()[0].message()
    );
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn sqlx_mysql_metadata_provider_reports_describe_failures_as_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = SqlxMysqlMetadataProvider::new(std::env::var(DATABASE_URL_ENV)?);
    let location = core::SourceLocation::at_position(
        "fixtures/sql/valid/missing.sql",
        core::SourcePosition::one_based(7, 1).expect("test position should be valid"),
    );
    let query = raw_query("SELECT missing_column FROM fixture_missing_table;")
        .with_source_location(location.clone());
    let analysis = core::AnalyzedQuery::new(core::Cardinality::Many);

    let report = provider
        .describe(&query, &analysis)
        .expect_err("missing table should produce a describe diagnostic");
    let diagnostic = &report.diagnostics()[0];

    assert!(
        diagnostic
            .message()
            .starts_with("failed to describe MySQL query:"),
        "{}",
        diagnostic.message()
    );
    assert_eq!(diagnostic.location(), Some(&location));

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn mysql_fixtures_load_and_describe_query_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let _fixture_lock = MYSQL_FIXTURE_LOCK
        .lock()
        .expect("fixture lock should not be poisoned");
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut connection = runtime.block_on(MySqlConnection::connect(&database_url))?;

    for fixture in INIT_FIXTURES {
        runtime.block_on(execute_fixture_statements(&mut connection, fixture))?;
    }

    let mut query_count = 0;
    let mut mapped_columns = Vec::new();
    for fixture in QUERY_FIXTURES {
        for sql in extract_sqlay_queries(fixture)? {
            query_count += 1;

            let description =
                runtime.block_on(connection.describe(AssertSqlSafe(sql.clone()).into_sql_str()))?;
            assert!(
                !description.columns().is_empty(),
                "query should expose columns: {sql}"
            );

            for column in description.columns() {
                assert!(
                    !column.name().is_empty(),
                    "query should expose non-empty column names: {sql}"
                );
            }

            mapped_columns.extend(description.columns().iter().enumerate().map(
                |(index, column)| {
                    map_mysql_result_column_metadata(
                        column.name(),
                        column.type_info().name(),
                        description.nullable(index),
                    )
                },
            ));
        }
    }

    assert!(
        query_count > 0,
        "query fixtures should contain @sqlay blocks"
    );
    assert_fixture_core_type_matrix(&mapped_columns);
    assert_fixture_nullability_matrix(&mapped_columns);

    Ok(())
}
