//! CLI driver boundary.
//!
//! The CLI is the composition root. It wires application ports to concrete
//! adapters and is the only crate that should depend on all adapter crates.

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
    self as app, ConfigLoader, DefaultCompilationPlanner, DefaultProjectInitializer,
    DefaultQueryCompiler,
};
use sqlcomp_core as core;

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
        Ok(Command::Init) => run_init_command(),
        Ok(Command::Check { config }) => run_configured_command("check", config),
        Ok(Command::Compile { config, clean: _ }) => run_configured_command("compile", config),
        Err(report) => fail(&report),
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Command {
    Noop,
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
        "init" => parse_no_options(args).map(|()| Command::Init),
        "check" => parse_options(args, CleanOption::Reject).map(|options| Command::Check {
            config: options.config,
        }),
        "compile" => parse_options(args, CleanOption::Allow).map(|options| Command::Compile {
            config: options.config,
            clean: options.clean,
        }),
        _ => Err(single_cli_error(format!(
            "unknown command `{}`; expected `init`, `check`, or `compile`",
            command.to_string_lossy()
        ))),
    }
}

fn run_configured_command(command: &str, config: Option<PathBuf>) -> ExitCode {
    let loader = config.map_or_else(
        JsoncConfigLoader::discover_from_current_dir,
        JsoncConfigLoader::new,
    );

    match loader.load() {
        Ok(_config) => fail(&single_cli_error(format!(
            "command `{command}` is not implemented yet"
        ))),
        Err(report) => fail(&report),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanOption {
    Allow,
    Reject,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct CommandOptions {
    config: Option<PathBuf>,
    clean: bool,
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
