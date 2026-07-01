use std::env::VarError;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use sqlay_adapters::config_jsonc::{JsoncConfigLoader, JsoncConfigTemplateWriter};
use sqlay_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlay_adapters::metadata::mysql::sqlx::SqlxMysqlMetadataProvider;
use sqlay_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlay_adapters::source_fs::FileSystemSourceReader;
use sqlay_adapters::target::typescript::TypeScriptTargetGenerator;
use sqlay_app::{
    self as app, CompilePipeline, ConfigLoader, DefaultCompilationPlanner, DefaultCompileUseCase,
    DefaultProjectInitializer, DefaultQueryCompiler, MetadataProvider, MutationMetadataProvider,
};
use sqlay_core as core;

use crate::args::{Command, ConfiguredCommand, OutputFormat, parse_args};
use crate::diagnostics::{fail, print_diagnostics, single_cli_error};
use crate::help::{INIT_NEXT_STEPS, help_text};
use crate::output::{ConfiguredCommandOutcome, print_json_failure_result, print_success_summary};

const EMPTY_SOURCE_SET_DIAGNOSTIC_PREFIX: &str =
    "source.include matched no SQL files after applying source.exclude";
const SKIPPED_EMPTY_CLEAN_DIAGNOSTIC_PREFIX: &str =
    "skipped stale generated file cleanup because no SQL files matched";

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

/// Run the `sqlay` command-line interface.
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
        Ok(Command::Check {
            config,
            format,
            fail_on_empty,
        }) => run_configured_command(
            ConfiguredCommand::Check {
                format,
                fail_on_empty,
            },
            config,
        ),
        Ok(Command::Compile {
            config,
            format,
            clean,
            fail_on_empty,
            allow_empty_clean,
        }) => run_configured_command(
            ConfiguredCommand::Compile {
                format,
                clean,
                fail_on_empty,
                allow_empty_clean,
            },
            config,
        ),
        Err(report) => fail(&report),
    }
}

fn run_configured_command(command: ConfiguredCommand, config: Option<PathBuf>) -> ExitCode {
    let config_path = config.clone();
    let loader = config.map_or_else(
        JsoncConfigLoader::discover_from_current_dir,
        JsoncConfigLoader::new,
    );

    let planner = DefaultCompilationPlanner;

    match loader.load().and_then(|config| {
        run_configured_use_case(command, &config, &planner)
            .map_err(|report| add_empty_source_cli_remediation(report, command))
    }) {
        Ok(outcome) => {
            let diagnostics = add_success_cli_remediation(outcome.diagnostics().clone(), command);
            print_diagnostics(&diagnostics);
            print_success_summary(&outcome);
            ExitCode::SUCCESS
        }
        Err(report) => fail_configured_command(command, config_path.as_deref(), &report),
    }
}

fn fail_configured_command(
    command: ConfiguredCommand,
    config_path: Option<&std::path::Path>,
    report: &core::DiagnosticReport,
) -> ExitCode {
    if command.format() == OutputFormat::Json {
        print_json_failure_result(command, config_path, report);
        ExitCode::FAILURE
    } else {
        fail(report)
    }
}

fn run_configured_use_case(
    command: ConfiguredCommand,
    config: &core::ProjectConfig,
    planner: &impl app::CompilationPlanner,
) -> core::DiagnosticResult<ConfiguredCommandOutcome> {
    let source_reader = FileSystemSourceReader;
    let dialect_analyzer = MysqlDialectAnalyzer;
    let metadata_provider = LazySqlxMysqlMetadataProvider::new(config.database());
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
        ConfiguredCommand::Check { fail_on_empty, .. } => {
            DefaultCompileUseCase::check_with_empty_source_policy(
                config,
                &pipeline,
                empty_source_policy(fail_on_empty),
            )
            .map(ConfiguredCommandOutcome::Check)
        }
        ConfiguredCommand::Compile {
            clean,
            fail_on_empty,
            allow_empty_clean,
            ..
        } => {
            let outcome = DefaultCompileUseCase::compile_with_empty_source_and_clean_policies(
                config,
                &pipeline,
                clean,
                empty_source_policy(fail_on_empty),
                empty_source_clean_policy(allow_empty_clean),
            )?;

            Ok(ConfiguredCommandOutcome::Compile(outcome))
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LazySqlxMysqlMetadataProvider<'a> {
    database: &'a core::DatabaseConfig,
}

impl<'a> LazySqlxMysqlMetadataProvider<'a> {
    const fn new(database: &'a core::DatabaseConfig) -> Self {
        Self { database }
    }

    fn provider(self) -> core::DiagnosticResult<SqlxMysqlMetadataProvider> {
        Ok(SqlxMysqlMetadataProvider::new(database_url_from_env(
            self.database,
        )?))
    }
}

impl MetadataProvider for LazySqlxMysqlMetadataProvider<'_> {
    fn describe(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        self.provider()?.describe(query, analysis)
    }
}

impl MutationMetadataProvider for LazySqlxMysqlMetadataProvider<'_> {
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata> {
        self.provider()?.describe_mutation(mutation, analysis)
    }
}

const fn empty_source_policy(fail_on_empty: bool) -> app::EmptySourceSetPolicy {
    if fail_on_empty {
        app::EmptySourceSetPolicy::Fail
    } else {
        app::EmptySourceSetPolicy::Warn
    }
}

const fn empty_source_clean_policy(allow_empty_clean: bool) -> app::EmptySourceSetCleanPolicy {
    if allow_empty_clean {
        app::EmptySourceSetCleanPolicy::Allow
    } else {
        app::EmptySourceSetCleanPolicy::Skip
    }
}

fn add_success_cli_remediation(
    mut report: core::DiagnosticReport,
    command: ConfiguredCommand,
) -> core::DiagnosticReport {
    if command.skips_empty_clean()
        && report.diagnostics().iter().any(|diagnostic| {
            diagnostic
                .message()
                .starts_with(SKIPPED_EMPTY_CLEAN_DIAGNOSTIC_PREFIX)
        })
    {
        report.push(core::Diagnostic::note(
            "pass `--allow-empty-clean` with `--clean` only when empty-source cleanup is intentional",
        ));
    }

    report
}

fn add_empty_source_cli_remediation(
    mut report: core::DiagnosticReport,
    command: ConfiguredCommand,
) -> core::DiagnosticReport {
    if !command.fail_on_empty()
        || !report.diagnostics().iter().any(|diagnostic| {
            diagnostic
                .message()
                .starts_with(EMPTY_SOURCE_SET_DIAGNOSTIC_PREFIX)
        })
    {
        return report;
    }

    report.push(core::Diagnostic::note(
        "disable `--fail-on-empty` only when an empty source set is intentional",
    ));
    report
}

impl ConfiguredCommand {
    const fn format(self) -> OutputFormat {
        match self {
            Self::Check { format, .. } | Self::Compile { format, .. } => format,
        }
    }

    const fn fail_on_empty(self) -> bool {
        match self {
            Self::Check { fail_on_empty, .. } | Self::Compile { fail_on_empty, .. } => {
                fail_on_empty
            }
        }
    }

    const fn skips_empty_clean(self) -> bool {
        matches!(
            self,
            Self::Compile {
                clean: true,
                allow_empty_clean: false,
                ..
            }
        )
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
                "failed to determine current directory while creating `sqlay.config.json`: {error}"
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
