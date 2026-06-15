//! CLI driver boundary.
//!
//! The CLI is the composition root. It wires application ports to concrete
//! adapters and is the only crate that should depend on all adapter crates.

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

const HELP: &str = "\
SQL Compose & Compile.

Usage:
  sqlcomp <command> [options]

Commands:
  sqlcomp init       Create a starter sqlcomp.config.json.
  sqlcomp check      Load config and run the compile pipeline without writing generated files.
  sqlcomp compile    Load config and write generated TypeScript files.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path for check or compile.
  --clean            Remove stale generated files during compile.
";

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
        Ok(Command::Noop) => ExitCode::SUCCESS,
        Ok(Command::Help) => {
            print!("{HELP}");
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

#[derive(Debug, Eq, PartialEq)]
enum Command {
    Noop,
    Help,
    Init,
    Check {
        config: Option<PathBuf>,
    },
    Compile {
        config: Option<PathBuf>,
        clean: bool,
    },
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> core::DiagnosticResult<Command> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Ok(Command::Noop);
    };

    match command.to_string_lossy().as_ref() {
        "--help" | "-h" | "help" => parse_no_options(args).map(|()| Command::Help),
        "init" => parse_init_args(args),
        "check" => parse_options(args, CleanOption::Reject).map(|options| {
            if options.help {
                Command::Help
            } else {
                Command::Check {
                    config: options.config,
                }
            }
        }),
        "compile" => parse_options(args, CleanOption::Allow).map(|options| {
            if options.help {
                Command::Help
            } else {
                Command::Compile {
                    config: options.config,
                    clean: options.clean,
                }
            }
        }),
        _ => Err(single_cli_error(format!(
            "unknown command `{}`; expected `init`, `check`, `compile`, or `--help`",
            command.to_string_lossy()
        ))),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfiguredCommand {
    Check,
    Compile { clean: bool },
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
        Ok(diagnostics) => {
            print_diagnostics(&diagnostics);
            ExitCode::SUCCESS
        }
        Err(report) => fail(&report),
    }
}

fn run_configured_use_case(
    command: ConfiguredCommand,
    config: &core::ProjectConfig,
    planner: &impl app::CompilationPlanner,
) -> core::DiagnosticResult<core::DiagnosticReport> {
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
        ConfiguredCommand::Check => DefaultCompileUseCase::check(config, &pipeline),
        ConfiguredCommand::Compile { clean } => {
            DefaultCompileUseCase::compile(config, &pipeline, clean)
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
        Ok(_path) => ExitCode::SUCCESS,
        Err(report) => fail(&report),
    }
}

fn fail(report: &core::DiagnosticReport) -> ExitCode {
    eprintln!("{report}");
    ExitCode::FAILURE
}

fn print_diagnostics(report: &core::DiagnosticReport) {
    if !report.is_empty() {
        eprintln!("{report}");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanOption {
    Allow,
    Reject,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct CommandOptions {
    config: Option<PathBuf>,
    clean: bool,
    help: bool,
}

fn parse_init_args(args: impl IntoIterator<Item = OsString>) -> core::DiagnosticResult<Command> {
    let mut args = args.into_iter();
    let Some(arg) = args.next() else {
        return Ok(Command::Init);
    };

    if is_help_arg(&arg) {
        parse_no_options(args).map(|()| Command::Help)
    } else {
        Err(unexpected_argument(&arg))
    }
}

fn parse_no_options(args: impl IntoIterator<Item = OsString>) -> core::DiagnosticResult<()> {
    let mut args = args.into_iter();
    if let Some(arg) = args.next() {
        return Err(unexpected_argument(&arg));
    }

    Ok(())
}

fn parse_options(
    args: impl IntoIterator<Item = OsString>,
    clean: CleanOption,
) -> core::DiagnosticResult<CommandOptions> {
    let mut options = CommandOptions::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if arg == "--config" {
            let Some(path) = args.next() else {
                return Err(single_cli_error("missing value for `--config`"));
            };

            if options.config.replace(PathBuf::from(path)).is_some() {
                return Err(single_cli_error("`--config` may only be provided once"));
            }
        } else if arg == "--clean" {
            if clean == CleanOption::Reject {
                return Err(unexpected_argument(&arg));
            }

            if options.clean {
                return Err(single_cli_error("`--clean` may only be provided once"));
            }

            options.clean = true;
        } else if is_help_arg(&arg) {
            options.help = true;
        } else if let Some(path) = arg
            .to_str()
            .and_then(|arg| arg.strip_prefix("--config="))
            .map(PathBuf::from)
        {
            if options.config.replace(path).is_some() {
                return Err(single_cli_error("`--config` may only be provided once"));
            }
        } else {
            return Err(unexpected_argument(&arg));
        }
    }

    Ok(options)
}

fn is_help_arg(arg: &OsString) -> bool {
    arg == "--help" || arg == "-h"
}

fn unexpected_argument(arg: &OsString) -> core::DiagnosticReport {
    single_cli_error(format!("unexpected argument `{}`", arg.to_string_lossy()))
}

fn single_cli_error(message: impl Into<String>) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(message))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{Command, parse_args};

    #[test]
    fn parses_check_without_config() {
        assert_eq!(
            parse_args(["sqlcomp", "check"].map(OsString::from)).expect("args should parse"),
            Command::Check { config: None }
        );
    }

    #[test]
    fn parses_help_flag() {
        assert_eq!(
            parse_args(["sqlcomp", "--help"].map(OsString::from)).expect("args should parse"),
            Command::Help
        );
    }

    #[test]
    fn parses_command_help_flags() {
        for args in [
            ["sqlcomp", "init", "--help"],
            ["sqlcomp", "check", "--help"],
            ["sqlcomp", "compile", "--help"],
        ] {
            assert_eq!(
                parse_args(args.map(OsString::from)).expect("args should parse"),
                Command::Help
            );
        }
    }

    #[test]
    fn parses_init_command() {
        assert_eq!(
            parse_args(["sqlcomp", "init"].map(OsString::from)).expect("args should parse"),
            Command::Init
        );
    }

    #[test]
    fn parses_compile_with_config_path() {
        assert_eq!(
            parse_args(
                [
                    "sqlcomp",
                    "compile",
                    "--config",
                    "custom/sqlcomp.config.json"
                ]
                .map(OsString::from)
            )
            .expect("args should parse"),
            Command::Compile {
                config: Some(PathBuf::from("custom/sqlcomp.config.json")),
                clean: false,
            }
        );
    }

    #[test]
    fn parses_compile_clean() {
        assert_eq!(
            parse_args(["sqlcomp", "compile", "--clean"].map(OsString::from))
                .expect("args should parse"),
            Command::Compile {
                config: None,
                clean: true,
            }
        );
    }

    #[test]
    fn parses_equals_form_config_path() {
        assert_eq!(
            parse_args(
                ["sqlcomp", "check", "--config=custom/sqlcomp.config.json"].map(OsString::from)
            )
            .expect("args should parse"),
            Command::Check {
                config: Some(PathBuf::from("custom/sqlcomp.config.json")),
            }
        );
    }

    #[test]
    fn rejects_missing_config_path() {
        let report = parse_args(["sqlcomp", "check", "--config"].map(OsString::from))
            .expect_err("missing config value should fail");

        assert_eq!(
            report.diagnostics()[0].message(),
            "missing value for `--config`"
        );
    }
}
