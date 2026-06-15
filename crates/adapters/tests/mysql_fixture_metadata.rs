use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::metadata_mysql_sqlx::{
    SqlxMysqlMetadataProvider, map_mysql_result_column_metadata,
};
use sqlcomp_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlcomp_adapters::source_fs::{FileSystemSourceReader, split_sqlcomp_query_blocks};
use sqlcomp_adapters::target_typescript::TypeScriptTargetGenerator;
use sqlcomp_app::{
    CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase, DefaultQueryCompiler,
    DialectAnalyzer, MetadataProvider,
};
use sqlcomp_core as core;
use sqlx::TypeInfo;
use sqlx::{Column, Connection, Executor, MySqlConnection, SqlSafeStr};

const DATABASE_URL_ENV: &str = "DATABASE_URL";
static MYSQL_FIXTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const INIT_FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/sql/schema.sql"),
    include_str!("../../../fixtures/sql/seed.sql"),
];

const QUERY_FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/sql/valid/type_metadata_matrix.sql"),
    include_str!("../../../fixtures/sql/valid/generation_surface.sql"),
    include_str!("../../../fixtures/sql/valid/nested/path_mapping.sql"),
];

const VALID_CONFIG: &str = include_str!("../../../fixtures/sql/sqlcomp.valid.config.json");
const INVALID_CONFIG: &str = include_str!("../../../fixtures/sql/sqlcomp.invalid.config.json");

struct FixtureColumnCoverage {
    nullable_name: &'static str,
    nullable_definition: &'static str,
    not_null_name: &'static str,
    not_null_definition: &'static str,
    core_type: core::CoreType,
}

const fn fixture_column_coverage(
    nullable_name: &'static str,
    nullable_definition: &'static str,
    not_null_name: &'static str,
    not_null_definition: &'static str,
    core_type: core::CoreType,
) -> FixtureColumnCoverage {
    FixtureColumnCoverage {
        nullable_name,
        nullable_definition,
        not_null_name,
        not_null_definition,
        core_type,
    }
}

static FIXTURE_ALL_COLUMN_TYPE_COVERAGE: &[FixtureColumnCoverage] = &[
    fixture_column_coverage(
        "tinyint_col",
        "tinyint_col TINYINT NULL",
        "tinyint_nn_col",
        "tinyint_nn_col TINYINT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "tinyint_unsigned_col",
        "tinyint_unsigned_col TINYINT UNSIGNED NULL",
        "tinyint_unsigned_nn_col",
        "tinyint_unsigned_nn_col TINYINT UNSIGNED NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "smallint_col",
        "smallint_col SMALLINT NULL",
        "smallint_nn_col",
        "smallint_nn_col SMALLINT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "smallint_unsigned_col",
        "smallint_unsigned_col SMALLINT UNSIGNED NULL",
        "smallint_unsigned_nn_col",
        "smallint_unsigned_nn_col SMALLINT UNSIGNED NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "mediumint_col",
        "mediumint_col MEDIUMINT NULL",
        "mediumint_nn_col",
        "mediumint_nn_col MEDIUMINT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "mediumint_unsigned_col",
        "mediumint_unsigned_col MEDIUMINT UNSIGNED NULL",
        "mediumint_unsigned_nn_col",
        "mediumint_unsigned_nn_col MEDIUMINT UNSIGNED NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "int_col",
        "int_col INT NULL",
        "int_nn_col",
        "int_nn_col INT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "int_unsigned_col",
        "int_unsigned_col INT UNSIGNED NULL",
        "int_unsigned_nn_col",
        "int_unsigned_nn_col INT UNSIGNED NOT NULL",
        core::CoreType::Int64,
    ),
    fixture_column_coverage(
        "integer_col",
        "integer_col INTEGER NULL",
        "integer_nn_col",
        "integer_nn_col INTEGER NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "bigint_col",
        "bigint_col BIGINT NULL",
        "bigint_nn_col",
        "bigint_nn_col BIGINT NOT NULL PRIMARY KEY",
        core::CoreType::Int64,
    ),
    fixture_column_coverage(
        "bigint_unsigned_col",
        "bigint_unsigned_col BIGINT UNSIGNED NULL",
        "bigint_unsigned_nn_col",
        "bigint_unsigned_nn_col BIGINT UNSIGNED NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "decimal_18_4_col",
        "decimal_18_4_col DECIMAL(18, 4) NULL",
        "decimal_18_4_nn_col",
        "decimal_18_4_nn_col DECIMAL(18, 4) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "dec_col",
        "dec_col DEC(12, 2) NULL",
        "dec_nn_col",
        "dec_nn_col DEC(12, 2) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "numeric_col",
        "numeric_col NUMERIC(12, 2) NULL",
        "numeric_nn_col",
        "numeric_nn_col NUMERIC(12, 2) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "fixed_col",
        "fixed_col FIXED(12, 2) NULL",
        "fixed_nn_col",
        "fixed_nn_col FIXED(12, 2) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "float_col",
        "float_col FLOAT NULL",
        "float_nn_col",
        "float_nn_col FLOAT NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "double_col",
        "double_col DOUBLE NULL",
        "double_nn_col",
        "double_nn_col DOUBLE NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "double_precision_col",
        "double_precision_col DOUBLE PRECISION NULL",
        "double_precision_nn_col",
        "double_precision_nn_col DOUBLE PRECISION NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "real_col",
        "real_col REAL NULL",
        "real_nn_col",
        "real_nn_col REAL NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "bit_col",
        "bit_col BIT(8) NULL",
        "bit_nn_col",
        "bit_nn_col BIT(8) NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "bool_col",
        "bool_col BOOL NULL",
        "bool_nn_col",
        "bool_nn_col BOOL NOT NULL",
        core::CoreType::Bool,
    ),
    fixture_column_coverage(
        "boolean_col",
        "boolean_col BOOLEAN NULL",
        "boolean_nn_col",
        "boolean_nn_col BOOLEAN NOT NULL",
        core::CoreType::Bool,
    ),
    fixture_column_coverage(
        "date_col",
        "date_col DATE NULL",
        "date_nn_col",
        "date_nn_col DATE NOT NULL",
        core::CoreType::Date,
    ),
    fixture_column_coverage(
        "time_col",
        "time_col TIME NULL",
        "time_nn_col",
        "time_nn_col TIME NOT NULL",
        core::CoreType::Time,
    ),
    fixture_column_coverage(
        "datetime_6_col",
        "datetime_6_col DATETIME(6) NULL",
        "datetime_6_nn_col",
        "datetime_6_nn_col DATETIME(6) NOT NULL",
        core::CoreType::DateTime,
    ),
    fixture_column_coverage(
        "timestamp_col",
        "timestamp_col TIMESTAMP NULL DEFAULT NULL",
        "timestamp_nn_col",
        "timestamp_nn_col TIMESTAMP NOT NULL",
        core::CoreType::DateTime,
    ),
    fixture_column_coverage(
        "year_col",
        "year_col YEAR NULL",
        "year_nn_col",
        "year_nn_col YEAR NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "char_16_col",
        "char_16_col CHAR(16) NULL",
        "char_16_nn_col",
        "char_16_nn_col CHAR(16) NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "varchar_255_col",
        "varchar_255_col VARCHAR(255) NULL",
        "varchar_255_nn_col",
        "varchar_255_nn_col VARCHAR(255) NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "varchar_320_col",
        "varchar_320_col VARCHAR(320) NULL",
        "varchar_320_nn_col",
        "varchar_320_nn_col VARCHAR(320) NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "tinytext_col",
        "tinytext_col TINYTEXT NULL",
        "tinytext_nn_col",
        "tinytext_nn_col TINYTEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "text_col",
        "text_col TEXT NULL",
        "text_nn_col",
        "text_nn_col TEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "mediumtext_col",
        "mediumtext_col MEDIUMTEXT NULL",
        "mediumtext_nn_col",
        "mediumtext_nn_col MEDIUMTEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "longtext_col",
        "longtext_col LONGTEXT NULL",
        "longtext_nn_col",
        "longtext_nn_col LONGTEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "enum_col",
        "enum_col ENUM('one', 'two') NULL",
        "enum_nn_col",
        "enum_nn_col ENUM('one', 'two') NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "set_col",
        "set_col SET('one', 'two') NULL",
        "set_nn_col",
        "set_nn_col SET('one', 'two') NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "binary_16_col",
        "binary_16_col BINARY(16) NULL",
        "binary_16_nn_col",
        "binary_16_nn_col BINARY(16) NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "varbinary_64_col",
        "varbinary_64_col VARBINARY(64) NULL",
        "varbinary_64_nn_col",
        "varbinary_64_nn_col VARBINARY(64) NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "tinyblob_col",
        "tinyblob_col TINYBLOB NULL",
        "tinyblob_nn_col",
        "tinyblob_nn_col TINYBLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "blob_col",
        "blob_col BLOB NULL",
        "blob_nn_col",
        "blob_nn_col BLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "mediumblob_col",
        "mediumblob_col MEDIUMBLOB NULL",
        "mediumblob_nn_col",
        "mediumblob_nn_col MEDIUMBLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "longblob_col",
        "longblob_col LONGBLOB NULL",
        "longblob_nn_col",
        "longblob_nn_col LONGBLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "json_col",
        "json_col JSON NULL",
        "json_nn_col",
        "json_nn_col JSON NOT NULL",
        core::CoreType::Json,
    ),
    fixture_column_coverage(
        "geometry_col",
        "geometry_col GEOMETRY NULL",
        "geometry_nn_col",
        "geometry_nn_col GEOMETRY NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "point_col",
        "point_col POINT NULL",
        "point_nn_col",
        "point_nn_col POINT NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "linestring_col",
        "linestring_col LINESTRING NULL",
        "linestring_nn_col",
        "linestring_nn_col LINESTRING NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "polygon_col",
        "polygon_col POLYGON NULL",
        "polygon_nn_col",
        "polygon_nn_col POLYGON NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "multipoint_col",
        "multipoint_col MULTIPOINT NULL",
        "multipoint_nn_col",
        "multipoint_nn_col MULTIPOINT NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "multilinestring_col",
        "multilinestring_col MULTILINESTRING NULL",
        "multilinestring_nn_col",
        "multilinestring_nn_col MULTILINESTRING NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "multipolygon_col",
        "multipolygon_col MULTIPOLYGON NULL",
        "multipolygon_nn_col",
        "multipolygon_nn_col MULTIPOLYGON NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "geometrycollection_col",
        "geometrycollection_col GEOMETRYCOLLECTION NULL",
        "geometrycollection_nn_col",
        "geometrycollection_nn_col GEOMETRYCOLLECTION NOT NULL",
        core::CoreType::Unknown,
    ),
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
    assert_fixture_nullability_matrix(&mapped_columns);

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn check_command_dry_runs_fixture_sql_without_writing_generated_files()
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

    let project_dir = unique_temp_dir("sqlcomp-check-mysql-fixture");
    let valid_dir = project_dir.join("valid");
    std::fs::create_dir_all(&valid_dir)?;
    std::fs::write(
        valid_dir.join("type_metadata_matrix.sql"),
        QUERY_FIXTURES[0],
    )?;

    let config = project_config(project_dir.clone());
    let metadata_provider = SqlxMysqlMetadataProvider::new(database_url);
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &FileSystemSourceReader,
        dialect_analyzer: &MysqlDialectAnalyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &DefaultQueryCompiler,
        target_generator: &TypeScriptTargetGenerator,
        generated_file_writer: &FileSystemGeneratedFileWriter,
    };
    DefaultCompileUseCase::check(&config, &pipeline)?;

    assert!(
        !project_dir.join("generated").exists(),
        "check must not create the configured output directory"
    );

    std::fs::remove_dir_all(project_dir)?;

    Ok(())
}

#[test]
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn compile_generates_one_typescript_module_for_multiple_queries_in_one_sql_file()
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

    let project_dir = unique_temp_dir("sqlcomp-compile-multiple-query-fixture");
    let sql_dir = project_dir.join("sql");
    std::fs::create_dir_all(&sql_dir)?;
    std::fs::write(sql_dir.join("business.sql"), QUERY_FIXTURES[1])?;

    let config = project_config(project_dir.clone());
    let metadata_provider = SqlxMysqlMetadataProvider::new(database_url);
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &FileSystemSourceReader,
        dialect_analyzer: &MysqlDialectAnalyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &DefaultQueryCompiler,
        target_generator: &TypeScriptTargetGenerator,
        generated_file_writer: &FileSystemGeneratedFileWriter,
    };
    DefaultCompileUseCase::compile(&config, &pipeline, false)?;

    let generated_dir = project_dir.join("src/generated/sqlcomp/sql");
    let generated_path = generated_dir.join("business.ts");
    let generated = std::fs::read_to_string(&generated_path)?;
    let generated_files = std::fs::read_dir(&generated_dir)?.collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        generated_files.len(),
        1,
        "one SQL file should generate one TypeScript module"
    );
    assert!(
        generated.starts_with(core::GENERATED_FILE_HEADER),
        "generated file should include the sqlcomp header"
    );

    let expected_queries = [
        ("businessOrderSummary", "businessOrderSummary_Row[]"),
        (
            "businessCustomerProfile",
            "businessCustomerProfile_Row | null",
        ),
        (
            "businessCustomerOrderLeftJoin",
            "businessCustomerOrderLeftJoin_Row[]",
        ),
        ("businessLineItemTotals", "businessLineItemTotals_Row[]"),
    ];

    for (id, output_type) in expected_queries {
        assert!(
            generated.contains(&format!("export type {id}_Input = Record<string, never>;")),
            "generated file should contain input type for `{id}`"
        );
        assert!(
            generated.contains(&format!("export type {id}_Row = {{")),
            "generated file should contain row type for `{id}`"
        );
        assert!(
            generated.contains(&format!("export type {id}_Output = {output_type};")),
            "generated file should contain output type for `{id}`"
        );
        assert!(
            generated.contains(&format!("export function {id}(")),
            "generated file should contain builder function for `{id}`"
        );
    }

    assert_eq!(
        generated.matches("export function ").count(),
        expected_queries.len(),
        "generated module should contain exactly the expected query builders"
    );

    std::fs::remove_dir_all(project_dir)?;

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

#[test]
fn mysql_fixtures_use_meta_schema_names() {
    assert!(
        INIT_FIXTURES[0].contains("CREATE TABLE fixture_all_column_type"),
        "schema should use a metadata-oriented parent table name"
    );
    assert!(
        INIT_FIXTURES[0].contains("CREATE TABLE fixture_child"),
        "schema should use a metadata-oriented child table name"
    );
    assert!(
        INIT_FIXTURES[0].contains("bigint_nn_col BIGINT NOT NULL PRIMARY KEY"),
        "schema should name columns by type/nullability metadata"
    );

    for fixture in INIT_FIXTURES.iter().chain(QUERY_FIXTURES) {
        for project_term in [
            "fixture_type_metadata_users",
            "fixture_type_metadata_orders",
            "display_name",
            "nickname",
            "email",
            "order_number",
            "typeMetadataSingleUser",
            "singleUser",
        ] {
            assert!(
                !fixture.contains(project_term),
                "fixture should not contain project-like term `{project_term}`"
            );
        }
    }
}

#[test]
fn mysql_fixtures_use_sql_valid_invalid_layout() {
    for required_path in [
        "fixtures/sql/sqlcomp.valid.config.json",
        "fixtures/sql/sqlcomp.invalid.config.json",
        "fixtures/sql/valid/type_metadata_matrix.sql",
        "fixtures/sql/valid/generation_surface.sql",
        "fixtures/sql/valid/nested/path_mapping.sql",
        "fixtures/sql/invalid/non_select.sql",
    ] {
        assert!(
            repo_path(required_path).exists(),
            "fixture path should exist: {required_path}"
        );
    }

    for legacy_path in [
        "fixtures/mysql/sqlcomp.config.json",
        "fixtures/mysql/queries/type_metadata_matrix.sql",
        "fixtures/sqlcomp/invalid/non_select.sql",
    ] {
        assert!(
            !repo_path(legacy_path).exists(),
            "legacy fixture path should be removed: {legacy_path}"
        );
    }

    assert!(VALID_CONFIG.contains(r#""include": ["valid/**/*.sql"]"#));
    assert!(INVALID_CONFIG.contains(r#""include": ["invalid/**/*.sql"]"#));
}

#[test]
fn fixture_all_column_type_schema_covers_mysql_type_categories_in_order() {
    let schema = INIT_FIXTURES[0];
    let actual_columns = fixture_all_column_type_columns(schema);
    let expected_columns = FIXTURE_ALL_COLUMN_TYPE_COVERAGE
        .iter()
        .flat_map(|column| {
            [
                format!("  {}", column.nullable_definition),
                format!("  {}", column.not_null_definition),
            ]
        })
        .collect::<Vec<_>>();

    assert_eq!(
        actual_columns, expected_columns,
        "fixture_all_column_type should list MySQL type categories in coverage order",
    );
}

#[test]
fn fixture_all_column_type_schema_covers_nullable_and_not_null_pairs() {
    let columns = fixture_all_column_type_columns(INIT_FIXTURES[0]);

    for column in FIXTURE_ALL_COLUMN_TYPE_COVERAGE {
        assert!(
            columns
                .iter()
                .any(|schema_column| schema_column.trim() == column.nullable_definition),
            "missing nullable fixture column `{}`",
            column.nullable_definition,
        );
        assert!(
            columns
                .iter()
                .any(|schema_column| schema_column.trim() == column.not_null_definition),
            "missing not-null fixture column `{}`",
            column.not_null_definition,
        );
    }
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
    assert_fixture_nullability_matrix(&mapped_columns);

    Ok(())
}

fn assert_fixture_core_type_matrix(columns: &[core::DbResultColumn]) {
    for column in FIXTURE_ALL_COLUMN_TYPE_COVERAGE {
        assert_mapped_type(columns, column.nullable_name, column.core_type);
        assert_mapped_type(columns, column.not_null_name, column.core_type);
    }

    assert_mapped_type(columns, "childTimeCol", core::CoreType::Time);
}

fn assert_fixture_nullability_matrix(columns: &[core::DbResultColumn]) {
    for column in FIXTURE_ALL_COLUMN_TYPE_COVERAGE {
        assert_mapped_nullability(columns, column.nullable_name, Some(true), true);
        assert_mapped_nullability(columns, column.not_null_name, Some(false), false);
    }
}

fn assert_mapped_type(columns: &[core::DbResultColumn], name: &str, expected_type: core::CoreType) {
    let column = columns
        .iter()
        .find(|column| column.name() == name)
        .unwrap_or_else(|| panic!("fixture should expose column `{name}`"));

    assert_eq!(column.ty(), expected_type, "{name} should map to core type");
}

fn assert_mapped_nullability(
    columns: &[core::DbResultColumn],
    name: &str,
    expected_metadata: Option<bool>,
    expected_output_nullable: bool,
) {
    let column = columns
        .iter()
        .find(|column| column.name() == name)
        .unwrap_or_else(|| panic!("fixture should expose column `{name}`"));

    assert_eq!(
        column.nullable(),
        expected_metadata,
        "{name} should preserve MySQL nullability metadata",
    );
    assert_eq!(
        column.to_result_column().is_nullable(),
        expected_output_nullable,
        "{name} should map to conservative Core IR output nullability",
    );
}

fn fixture_all_column_type_columns(schema: &str) -> Vec<String> {
    let start_marker = "CREATE TABLE fixture_all_column_type (\n";
    let start = schema
        .find(start_marker)
        .expect("schema should define fixture_all_column_type")
        + start_marker.len();
    let end = schema[start..]
        .find("\n);")
        .expect("fixture_all_column_type definition should be closed")
        + start;

    schema[start..end]
        .lines()
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

fn repo_path(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
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

fn project_config(config_dir: std::path::PathBuf) -> core::ProjectConfig {
    core::ProjectConfig::new(
        config_dir,
        core::SourceConfig::new(vec!["valid/**/*.sql".to_owned()], Vec::new()),
        core::OutputConfig::new("generated".to_owned()),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, DATABASE_URL_ENV.to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    )
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
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
