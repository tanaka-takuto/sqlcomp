use std::ffi::OsString;
use std::path::PathBuf;

use sqlcomp_core as core;

use crate::diagnostics::single_cli_error;
use crate::help::HelpTopic;

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Help(HelpTopic),
    Init,
    Check {
        config: Option<PathBuf>,
    },
    Compile {
        config: Option<PathBuf>,
        clean: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfiguredCommand {
    Check,
    Compile { clean: bool },
}

pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> core::DiagnosticResult<Command> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Ok(Command::Help(HelpTopic::TopLevel));
    };

    match command.to_string_lossy().as_ref() {
        "--help" | "-h" | "help" => {
            parse_no_options(args).map(|()| Command::Help(HelpTopic::TopLevel))
        }
        "init" => parse_init_args(args),
        "check" => parse_options(args, CleanOption::Reject).map(|options| {
            if options.help {
                Command::Help(HelpTopic::Check)
            } else {
                Command::Check {
                    config: options.config,
                }
            }
        }),
        "compile" => parse_options(args, CleanOption::Allow).map(|options| {
            if options.help {
                Command::Help(HelpTopic::Compile)
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
        parse_no_options(args).map(|()| Command::Help(HelpTopic::Init))
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use crate::args::{Command, parse_args};
    use crate::help::HelpTopic;

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
            Command::Help(HelpTopic::TopLevel)
        );
    }

    #[test]
    fn parses_no_args_as_top_level_help() {
        assert_eq!(
            parse_args(["sqlcomp"].map(OsString::from)).expect("args should parse"),
            Command::Help(HelpTopic::TopLevel)
        );
    }

    #[test]
    fn parses_command_help_flags() {
        for (args, expected) in [
            (["sqlcomp", "init", "--help"], HelpTopic::Init),
            (["sqlcomp", "check", "--help"], HelpTopic::Check),
            (["sqlcomp", "compile", "--help"], HelpTopic::Compile),
        ] {
            assert_eq!(
                parse_args(args.map(OsString::from)).expect("args should parse"),
                Command::Help(expected)
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
                    "custom/sqlcomp.config.json",
                ]
                .map(OsString::from),
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
                ["sqlcomp", "check", "--config=custom/sqlcomp.config.json"].map(OsString::from),
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
