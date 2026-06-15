use sqlx::{Column, Connection, Executor, MySqlConnection, SqlSafeStr, TypeInfo};

const DATABASE_URL_ENV: &str = "DATABASE_URL";

const DROP_FIXTURE: &str = "DROP TABLE IF EXISTS fixture_metadata_spike_users;";
const CREATE_FIXTURE: &str = r"
CREATE TABLE fixture_metadata_spike_users (
  id BIGINT NOT NULL PRIMARY KEY,
  display_name VARCHAR(255) NOT NULL,
  nickname VARCHAR(255) NULL,
  age INT NULL,
  created_at DATETIME NOT NULL,
  deleted_at DATETIME NULL,
  balance DECIMAL(10, 2) NOT NULL,
  active TINYINT(1) NOT NULL,
  payload JSON NULL
);
";

const CASES: &[Case] = &[
    Case {
        name: "aliases_and_table_columns",
        sql: r"
SELECT
  id AS userId,
  display_name AS displayName,
  nickname AS nickname,
  created_at AS createdAt,
  deleted_at AS deletedAt
FROM fixture_metadata_spike_users;
",
        expected: &[
            ExpectedColumn::exact("userId", "BIGINT", Some(false)),
            ExpectedColumn::exact("displayName", "VARCHAR", Some(false)),
            ExpectedColumn::exact("nickname", "VARCHAR", Some(true)),
            ExpectedColumn::exact("createdAt", "DATETIME", Some(false)),
            ExpectedColumn::exact("deletedAt", "DATETIME", Some(true)),
        ],
    },
    Case {
        name: "expressions",
        sql: r"
SELECT
  id + 1 AS nextId,
  CONCAT(display_name, ':', id) AS label,
  age + 1 AS nextAge,
  deleted_at IS NULL AS isActiveExpression
FROM fixture_metadata_spike_users;
",
        expected: &[
            ExpectedColumn::exact("nextId", "BIGINT", Some(false)),
            ExpectedColumn::exact("label", "VARCHAR", Some(true)),
            ExpectedColumn::exact("nextAge", "BIGINT", Some(true)),
            ExpectedColumn::exact("isActiveExpression", "BIGINT", Some(false)),
        ],
    },
    Case {
        name: "mixed_database_types",
        sql: r"
SELECT
  balance AS balance,
  active AS active,
  payload AS payload
FROM fixture_metadata_spike_users;
",
        expected: &[
            ExpectedColumn::exact("balance", "DECIMAL", Some(false)),
            ExpectedColumn::exact("active", "BOOLEAN", Some(false)),
            ExpectedColumn::exact("payload", "JSON", Some(true)),
        ],
    },
];

#[tokio::test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
async fn sqlx_describes_mysql_statement_metadata_without_fetching_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var(DATABASE_URL_ENV)?;
    let mut connection = MySqlConnection::connect(&database_url).await?;

    reset_fixture(&mut connection).await?;

    for case in CASES {
        let actual = describe_columns(&mut connection, case.sql).await?;
        assert_columns(case, &actual);

        eprintln!("case: {}", case.name);
        for column in actual {
            eprintln!(
                "  {}: type={}, nullable={:?}",
                column.name, column.type_name, column.nullable
            );
        }
    }

    Ok(())
}

async fn reset_fixture(connection: &mut MySqlConnection) -> sqlx::Result<()> {
    sqlx::raw_sql(DROP_FIXTURE)
        .execute(&mut *connection)
        .await?;
    sqlx::raw_sql(CREATE_FIXTURE).execute(connection).await?;
    Ok(())
}

async fn describe_columns(
    connection: &mut MySqlConnection,
    sql: &'static str,
) -> sqlx::Result<Vec<ActualColumn>> {
    let description = connection.describe(sql.into_sql_str()).await?;

    Ok(description
        .columns()
        .iter()
        .enumerate()
        .map(|(index, column)| ActualColumn {
            name: column.name().to_owned(),
            type_name: column.type_info().name().to_owned(),
            nullable: description.nullable(index),
        })
        .collect())
}

fn assert_columns(case: &Case, actual: &[ActualColumn]) {
    assert_eq!(
        actual.len(),
        case.expected.len(),
        "{} column count mismatch",
        case.name
    );

    for (index, (actual, expected)) in actual.iter().zip(case.expected).enumerate() {
        assert_eq!(
            actual.name, expected.name,
            "{} column {index} name mismatch",
            case.name
        );

        if let Some(type_name) = expected.type_name {
            assert_eq!(
                actual.type_name, type_name,
                "{} column {index} type mismatch",
                case.name
            );
        } else {
            assert!(
                !actual.type_name.is_empty(),
                "{} column {index} should expose a database type",
                case.name
            );
        }

        if let Some(nullable) = expected.nullable {
            assert_eq!(
                actual.nullable,
                Some(nullable),
                "{} column {index} nullability mismatch",
                case.name
            );
        } else {
            assert!(
                actual.nullable.is_some(),
                "{} column {index} should expose nullability metadata",
                case.name
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Case {
    name: &'static str,
    sql: &'static str,
    expected: &'static [ExpectedColumn],
}

#[derive(Clone, Copy, Debug)]
struct ExpectedColumn {
    name: &'static str,
    type_name: Option<&'static str>,
    nullable: Option<bool>,
}

impl ExpectedColumn {
    const fn exact(name: &'static str, type_name: &'static str, nullable: Option<bool>) -> Self {
        Self {
            name,
            type_name: Some(type_name),
            nullable,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActualColumn {
    name: String,
    type_name: String,
    nullable: Option<bool>,
}
