use std::env::VarError;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use sqlcomp_adapters::config_jsonc::{JsoncConfigLoader, JsoncConfigTemplateWriter};
use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::metadata_mysql_sqlx::SqlxMysqlMetadataProvider;
use sqlcomp_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlcomp_adapters::source_fs::FileSystemSourceReader;
use sqlcomp_adapters::target_typescript::TypeScriptTargetGenerator;
use sqlcomp_app::{
    self as app, CompilePipeline, ConfigLoader, DefaultCompilationPlanner, DefaultCompileUseCase,
    DefaultProjectInitializer, DefaultQueryCompiler,
};
use sqlcomp_core as core;

use crate::args::{Command, ConfiguredCommand, parse_args};
use crate::diagnostics::{fail, print_diagnostics, single_cli_error};
use crate::help::{INIT_NEXT_STEPS, help_text};
use crate::output::{ConfiguredCommandOutcome, print_success_summary};

/// Default CLI composition root.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultPipeline;

impl app::CompileUseCasePorts for DefaultPipeline {
    type ConfigLoader = JsoncConfigLoader;
    type CompilationPlanner = DefaultCompilationPlanner;
    type SourceReader = FileSystemSourceReader;
    type DialectAnalyzer = MysqlDialectAnalyzer;
    type MetadataProvider = SqlxMysqlMetadataProvider;
    type QueryCompiler = DefaultQueryCompiler;
    type TargetGenerator = TypeScriptTargetGenerator;
    type GeneratedFileWriter = FileSystemGeneratedFileWriter;
}

/// Run the `sqlcomp` command-line interface.
#[must_use]
pub fn run() -> ExitCode {
    run_with_args(std::env::args_os())
}

fn run_with_args(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    match parse_args(args) {
        Ok(Command::Help(topic)) => {
            print!("{}", help_text(topic));
            ExitCode::SUCCESS
        }
        Ok(Command::Init) => run_init_command(),
        Ok(Command::Check { config }) => run_configured_command(ConfiguredCommand::Check, config),
        Ok(Command::Compile { config, clean }) => {
            run_configured_command(ConfiguredCommand::Compile { clean }, config)
        }
        Err(report) => fail(&report),
    }
}

fn run_configured_command(command: ConfiguredCommand, config: Option<PathBuf>) -> ExitCode {
    let loader = config.map_or_else(
        JsoncConfigLoader::discover_from_current_dir,
        JsoncConfigLoader::new,
    );

    let planner = DefaultCompilationPlanner;

    match loader
        .load()
        .and_then(|config| run_configured_use_case(command, &config, &planner))
    {
        Ok(outcome) => {
            print_diagnostics(outcome.diagnostics());
            print_success_summary(&outcome);
            ExitCode::SUCCESS
        }
        Err(report) => fail(&report),
    }
}

fn run_configured_use_case(
    command: ConfiguredCommand,
    config: &core::ProjectConfig,
    planner: &impl app::CompilationPlanner,
) -> core::DiagnosticResult<ConfiguredCommandOutcome> {
    let source_reader = FileSystemSourceReader;
    let dialect_analyzer = MysqlDialectAnalyzer;
    let database_url = database_url_from_env(config.database())?;
    let metadata_provider = SqlxMysqlMetadataProvider::new(database_url);
    let query_compiler = DefaultQueryCompiler;
    let target_generator = TypeScriptTargetGenerator;
    let generated_file_writer = FileSystemGeneratedFileWriter;
    let pipeline = CompilePipeline {
        planner,
        source_reader: &source_reader,
        dialect_analyzer: &dialect_analyzer,
        metadata_provider: &metadata_provider,
        query_compiler: &query_compiler,
        target_generator: &target_generator,
        generated_file_writer: &generated_file_writer,
    };

    match command {
        ConfiguredCommand::Check => {
            DefaultCompileUseCase::check(config, &pipeline).map(ConfiguredCommandOutcome::Check)
        }
        ConfiguredCommand::Compile { clean } => {
            let outcome = DefaultCompileUseCase::compile(config, &pipeline, clean)?;

            Ok(ConfiguredCommandOutcome::Compile(outcome))
        }
    }
}

fn database_url_from_env(database: &core::DatabaseConfig) -> core::DiagnosticResult<String> {
    let env_name = database.url_env();

    match std::env::var(env_name) {
        Ok(value) if value.is_empty() => Err(single_cli_error(format!(
            "environment variable `{env_name}` configured by `database.urlEnv` is empty"
        ))),
        Ok(value) => Ok(value),
        Err(VarError::NotPresent) => Err(single_cli_error(format!(
            "environment variable `{env_name}` configured by `database.urlEnv` is not set"
        ))),
        Err(VarError::NotUnicode(_)) => Err(single_cli_error(format!(
            "environment variable `{env_name}` configured by `database.urlEnv` is not valid Unicode"
        ))),
    }
}

fn run_init_command() -> ExitCode {
    let current_dir = match std::env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(error) => {
            return fail(&single_cli_error(format!(
                "failed to determine current directory while creating `sqlcomp.config.json`: {error}"
            )));
        }
    };

    match DefaultProjectInitializer::init(&current_dir, &JsoncConfigTemplateWriter) {
        Ok(_path) => {
            println!("Created {}", app::CONFIG_FILE_NAME);
            print!("{INIT_NEXT_STEPS}");
            ExitCode::SUCCESS
        }
        Err(report) => fail(&report),
    }
}
