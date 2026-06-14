use sqlx::{Column, Connection, Executor, MySqlConnection, SqlSafeStr};

const DATABASE_URL_ENV: &str = "DATABASE_URL";

const INIT_FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/mysql/init/001_metadata_fixture.sql"),
    include_str!("../../../fixtures/mysql/init/002_business_fixture.sql"),
];

const QUERY_FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/mysql/queries/metadata.sql"),
    include_str!("../../../fixtures/mysql/queries/business.sql"),
];

#[tokio::test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
async fn mysql_fixtures_load_and_describe_query_metadata() -> Result<(), Box<dyn std::error::Error>>
{
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let mut connection = MySqlConnection::connect(&database_url).await?;

    for fixture in INIT_FIXTURES {
        execute_fixture_statements(&mut connection, fixture).await?;
    }

    let mut query_count = 0;
    for fixture in QUERY_FIXTURES {
        for sql in extract_sqlcomp_queries(fixture) {
            query_count += 1;

            let description = connection.describe(sql.into_sql_str()).await?;
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
        }
    }

    assert!(
        query_count > 0,
        "query fixtures should contain @sqlcomp blocks"
    );

    Ok(())
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
