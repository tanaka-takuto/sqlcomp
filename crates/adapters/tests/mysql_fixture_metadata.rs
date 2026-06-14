use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::metadata_mysql_sqlx::{
    SqlxMysqlMetadataProvider, map_mysql_result_column_metadata,
};
use sqlcomp_adapters::source_fs::split_sqlcomp_query_blocks;
use sqlcomp_app::{DialectAnalyzer, MetadataProvider};
use sqlcomp_core as core;
use sqlx::TypeInfo;
use sqlx::{Column, Connection, Executor, MySqlConnection, SqlSafeStr};

const DATABASE_URL_ENV: &str = "DATABASE_URL";
static MYSQL_FIXTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const INIT_FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/mysql/init/001_metadata_fixture.sql"),
    include_str!("../../../fixtures/mysql/init/002_business_fixture.sql"),
];

const QUERY_FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/mysql/queries/metadata.sql"),
    include_str!("../../../fixtures/mysql/queries/business.sql"),
];

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
        for query in split_sqlcomp_query_blocks(fixture)? {
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
        "query fixtures should contain @sqlcomp blocks"
    );
    assert_fixture_core_type_matrix(&mapped_columns);

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
        "fixtures/mysql/queries/missing.sql",
        core::SourcePosition::one_based(7, 1).expect("test position should be valid"),
    );
    let query = raw_query("SELECT missing_column FROM sqlcomp_missing_table;")
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
        for sql in extract_sqlcomp_queries(fixture) {
            query_count += 1;

            let description = runtime.block_on(connection.describe(sql.into_sql_str()))?;
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
        "query fixtures should contain @sqlcomp blocks"
    );
    assert_fixture_core_type_matrix(&mapped_columns);

    Ok(())
}

fn assert_fixture_core_type_matrix(columns: &[core::DbResultColumn]) {
    assert_mapped_type(columns, "userId", core::CoreType::Int64);
    assert_mapped_type(columns, "displayName", core::CoreType::String);
    assert_mapped_type(columns, "accountBalance", core::CoreType::Decimal);
    assert_mapped_type(columns, "ratioFloat", core::CoreType::Float64);
    assert_mapped_type(columns, "scoreDouble", core::CoreType::Float64);
    assert_mapped_type(columns, "avatarBytes", core::CoreType::Bytes);
    assert_mapped_type(columns, "profileBlob", core::CoreType::Bytes);
    assert_mapped_type(columns, "birthDate", core::CoreType::Date);
    assert_mapped_type(columns, "createdAt", core::CoreType::DateTime);
    assert_mapped_type(columns, "lastSeenAt", core::CoreType::DateTime);
    assert_mapped_type(columns, "deliveryWindow", core::CoreType::Time);
    assert_mapped_type(columns, "active", core::CoreType::Bool);
    assert_mapped_type(columns, "settings", core::CoreType::Json);
}

fn assert_mapped_type(columns: &[core::DbResultColumn], name: &str, expected_type: core::CoreType) {
    let column = columns
        .iter()
        .find(|column| column.name() == name)
        .unwrap_or_else(|| panic!("fixture should expose column `{name}`"));

    assert_eq!(column.ty(), expected_type, "{name} should map to core type");
}

async fn execute_fixture_statements(
    connection: &mut MySqlConnection,
    fixture: &'static str,
) -> sqlx::Result<()> {
    for statement in fixture
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .filter(|statement| !is_connection_setup_statement(statement))
    {
        sqlx::raw_sql(statement).execute(&mut *connection).await?;
    }

    Ok(())
}

fn is_connection_setup_statement(statement: &str) -> bool {
    statement.starts_with("CREATE DATABASE ") || statement.starts_with("USE ")
}

fn raw_query(sql: &str) -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("testQuery".to_owned(), None),
        sql.to_owned(),
    )
}

fn extract_sqlcomp_queries(fixture: &'static str) -> Vec<&'static str> {
    let mut queries = Vec::new();
    let mut remaining = fixture;

    while let Some(annotation_start) = remaining.find("/* @sqlcomp") {
        let after_annotation = &remaining[annotation_start..];
        let Some(comment_end) = after_annotation.find("*/") else {
            break;
        };

        let after_comment = &after_annotation[comment_end + "*/".len()..];
        let next_annotation = after_comment
            .find("/* @sqlcomp")
            .unwrap_or(after_comment.len());
        let sql = after_comment[..next_annotation].trim();

        if !sql.is_empty() {
            queries.push(sql);
        }

        remaining = &after_comment[next_annotation..];
    }

    queries
}

#[cfg(test)]
mod tests {
    use super::extract_sqlcomp_queries;

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
        );

        assert_eq!(queries, vec!["SELECT 1;", "SELECT 2;"]);
    }
}
