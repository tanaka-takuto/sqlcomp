use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::metadata::mysql::sqlx::SqlxMysqlMetadataProvider;
use sqlcomp_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlcomp_adapters::source_fs::{FileSystemSourceReader, split_sqlcomp_query_blocks};
use sqlcomp_adapters::target::typescript::TypeScriptTargetGenerator;
use sqlcomp_app::{
    CompilePipeline, DefaultCompilationPlanner, DefaultCompileUseCase, DefaultQueryCompiler,
};
use sqlcomp_core as core;
use sqlx::MySqlConnection;

pub(super) const DATABASE_URL_ENV: &str = "DATABASE_URL";
pub(super) static MYSQL_FIXTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) const INIT_FIXTURES: &[&str] = &[
    include_str!("../../../../../fixtures/sql/schema.sql"),
    include_str!("../../../../../fixtures/sql/seed.sql"),
];

pub(super) const QUERY_FIXTURES: &[&str] = &[
    include_str!("../../../../../fixtures/sql/valid/type_metadata_matrix.sql"),
    include_str!("../../../../../fixtures/sql/valid/generation_surface.sql"),
    include_str!("../../../../../fixtures/sql/valid/nested/path_mapping.sql"),
    include_str!("../../../../../fixtures/sql/valid/param_bindings.sql"),
    include_str!("../../../../../fixtures/sql/valid/slot_runtime.sql"),
];

pub(super) const VALID_CONFIG: &str =
    include_str!("../../../../../fixtures/sql/sqlcomp.valid.config.json");
pub(super) const INVALID_CONFIG: &str =
    include_str!("../../../../../fixtures/sql/sqlcomp.invalid.config.json");
pub(super) const FRAGMENT_PARAM_INFERENCE_FAILURE: &str =
    include_str!("../../../../../fixtures/sql/invalid/fragment_param_inference_failure.sql");
pub(super) const PARAM_CONFLICTING_REPEATED_NULLABILITY: &str =
    include_str!("../../../../../fixtures/sql/invalid/param_conflicting_repeated_nullability.sql");
pub(super) const PARAM_CONFLICTING_REPEATED_TYPE: &str =
    include_str!("../../../../../fixtures/sql/invalid/param_conflicting_repeated_type.sql");
pub(super) const PARAM_UNSUPPORTED_INFERENCE_CONTEXT: &str =
    include_str!("../../../../../fixtures/sql/invalid/param_unsupported_inference_context.sql");
pub(super) const REPEATED_SLOT_FRAGMENT_PARAM_TYPE_CONFLICT: &str = include_str!(
    "../../../../../fixtures/sql/invalid/repeated_slot_fragment_param_type_conflict.sql"
);
pub(super) const SLOT_VARIANT_ROW_SHAPE_MISMATCH: &str =
    include_str!("../../../../../fixtures/sql/invalid/slot_variant_row_shape_mismatch.sql");
pub(super) const EXPECTED_GENERATION_SURFACE: &str =
    include_str!("../../../../../fixtures/sql/generated/valid/generation_surface.ts");
pub(super) const EXPECTED_NESTED_PATH_MAPPING: &str =
    include_str!("../../../../../fixtures/sql/generated/valid/nested/path_mapping.ts");

pub(super) fn repo_path(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

pub(super) async fn execute_fixture_statements(
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

pub(super) fn raw_query(sql: &str) -> core::RawQuery {
    core::RawQuery::new(
        core::QueryMetadata::new("testQuery".to_owned(), None),
        sql.to_owned(),
    )
}

pub(super) fn project_config(config_dir: std::path::PathBuf) -> core::ProjectConfig {
    core::ProjectConfig::new(
        config_dir,
        core::SourceConfig::new(vec!["valid/**/*.sql".to_owned()], Vec::new()),
        core::OutputConfig::new("generated".to_owned()),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, DATABASE_URL_ENV.to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    )
}

pub(super) fn assert_mysql_invalid_fixture_error_contains(
    database_url: &str,
    file_name: &str,
    source: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = unique_temp_dir("sqlcomp-invalid-param-fixture");
    let valid_dir = project_dir.join("valid");
    std::fs::create_dir_all(&valid_dir)?;
    std::fs::write(valid_dir.join(file_name), source)?;

    let config = project_config(project_dir.clone());
    let metadata_provider = SqlxMysqlMetadataProvider::new(database_url.to_owned());
    let pipeline = CompilePipeline {
        planner: &DefaultCompilationPlanner,
        source_reader: &FileSystemSourceReader,
        dialect_analyzer: &MysqlDialectAnalyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &DefaultQueryCompiler,
        target_generator: &TypeScriptTargetGenerator,
        generated_file_writer: &FileSystemGeneratedFileWriter,
    };
    let report = DefaultCompileUseCase::check(&config, &pipeline)
        .expect_err("invalid fixture should fail the compile pipeline");
    let messages = diagnostic_messages(&report);

    assert!(
        messages.contains(expected),
        "expected diagnostic containing `{expected}`, got:\n{messages}"
    );

    std::fs::remove_dir_all(project_dir)?;

    Ok(())
}

fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
    report
        .diagnostics()
        .iter()
        .map(core::Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn generated_relative_files(
    root: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_generated_relative_files(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_generated_relative_files(
    root: &std::path::Path,
    directory: &std::path::Path,
    files: &mut Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();

        if path.is_dir() {
            collect_generated_relative_files(root, &path, files)?;
        } else if path.is_file() {
            files.push(path.strip_prefix(root)?.to_path_buf());
        }
    }

    Ok(())
}

pub(super) fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

pub(super) fn extract_sqlcomp_queries(
    fixture: &'static str,
) -> core::DiagnosticResult<Vec<String>> {
    Ok(split_sqlcomp_query_blocks(fixture)?
        .into_iter()
        .map(|query| query.analysis_sql().trim().to_owned())
        .collect())
}
