use sqlay_adapters::config_jsonc::JsoncConfigLoader;
use sqlay_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlay_adapters::metadata::mysql::sqlx::SqlxMysqlMetadataProvider;
use sqlay_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlay_adapters::source_fs::FileSystemSourceReader;
use sqlay_adapters::target::typescript::TypeScriptTargetGenerator;
use sqlay_app::{
    CompilePipeline, ConfigLoader, DefaultCompilationPlanner, DefaultCompileUseCase,
    DefaultQueryCompiler,
};
use sqlay_core as core;
use sqlx::{Connection, MySqlConnection};

use super::fixture_support::{
    DATABASE_URL_ENV, EXPECTED_GENERATION_SURFACE, EXPECTED_NESTED_PATH_MAPPING, INIT_FIXTURES,
    MYSQL_FIXTURE_LOCK, QUERY_FIXTURES, VALID_CONFIG, execute_fixture_statements,
    generated_relative_files, project_config, unique_temp_dir,
};

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

    let project_dir = unique_temp_dir("sqlay-check-mysql-fixture");
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

    let project_dir = unique_temp_dir("sqlay-compile-multiple-query-fixture");
    let valid_dir = project_dir.join("valid");
    std::fs::create_dir_all(&valid_dir)?;
    std::fs::write(valid_dir.join("generation_surface.sql"), QUERY_FIXTURES[1])?;

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

    let generated_dir = project_dir.join("generated/valid");
    let generated_path = generated_dir.join("generation_surface.ts");
    let generated = std::fs::read_to_string(&generated_path)?;
    let generated_files = std::fs::read_dir(&generated_dir)?.collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        generated_files.len(),
        1,
        "one SQL file should generate one TypeScript module"
    );
    assert!(
        generated.starts_with(core::GENERATED_FILE_HEADER),
        "generated file should include the sqlay header"
    );

    let expected_queries = [
        ("generationEscapedSql", "generationEscapedSql_Row[]"),
        (
            "generationInferredSingleRow",
            "generationInferredSingleRow_Row | null",
        ),
        (
            "generationExplicitOneOverridesMany",
            "generationExplicitOneOverridesMany_Row | null",
        ),
        (
            "generationExplicitManyOverridesLimitOne",
            "generationExplicitManyOverridesLimitOne_Row[]",
        ),
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
#[ignore = "requires a running MySQL service and DATABASE_URL"]
fn compile_preserves_config_relative_paths_for_multiple_sql_files_from_nested_directory()
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

    let project_dir = unique_temp_dir("sqlay-compile-config-relative-path-fixture");
    let nested_current_dir = project_dir.join("valid/nested");
    std::fs::create_dir_all(&nested_current_dir)?;
    std::fs::write(project_dir.join("sqlay.config.json"), VALID_CONFIG)?;
    std::fs::write(
        project_dir.join("valid/generation_surface.sql"),
        QUERY_FIXTURES[1],
    )?;
    std::fs::write(
        nested_current_dir.join("path_mapping.sql"),
        QUERY_FIXTURES[2],
    )?;

    let config = JsoncConfigLoader::discover_from(&nested_current_dir).load()?;
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

    assert_eq!(
        config.config_dir(),
        project_dir.as_path(),
        "config discovery should resolve paths from sqlay.config.json, not the nested start directory"
    );
    assert_eq!(
        generated_relative_files(&project_dir.join("generated"))?,
        vec![
            std::path::PathBuf::from("valid/generation_surface.ts"),
            std::path::PathBuf::from("valid/nested/path_mapping.ts"),
        ],
        "multiple SQL inputs should preserve config-relative paths under output.dir"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("generated/valid/generation_surface.ts"))?,
        EXPECTED_GENERATION_SURFACE
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("generated/valid/nested/path_mapping.ts"))?,
        EXPECTED_NESTED_PATH_MAPPING
    );
    assert!(
        !nested_current_dir.join("generated").exists(),
        "nested start directory must not become the generated output base"
    );

    std::fs::remove_dir_all(project_dir)?;

    Ok(())
}
