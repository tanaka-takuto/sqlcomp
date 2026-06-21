use std::ffi::OsString;
use std::path::PathBuf;

use sqlay_core as core;

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
    let Some(first_arg) = args.next() else {
        return Ok(Command::Help(HelpTopic::TopLevel));
    };
    let (command, config) = parse_leading_config(first_arg, &mut args)?;

    match command.to_string_lossy().as_ref() {
        "--help" | "-h" | "help" if config.is_none() => {
            parse_no_options(args).map(|()| Command::Help(HelpTopic::TopLevel))
        }
        "--help" | "-h" | "help" => Err(config_before_unsupported_command()),
        "init" if config.is_some() => Err(config_before_unsupported_command()),
        "init" => parse_init_args(args),
        "check" => parse_options(args, CleanOption::Reject, config).map(|options| {
            if options.help {
                Command::Help(HelpTopic::Check)
            } else {
                Command::Check {
                    config: options.config,
                }
            }
        }),
        "compile" => parse_options(args, CleanOption::Allow, config).map(|options| {
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

fn parse_leading_config(
    first_arg: OsString,
    args: &mut impl Iterator<Item = OsString>,
) -> core::DiagnosticResult<(OsString, Option<PathBuf>)> {
    if first_arg == "--config" {
        let Some(path) = args.next() else {
            return Err(single_cli_error("missing value for `--config`"));
        };
        let Some(command) = args.next() else {
            return Err(single_cli_error("missing command after `--config`"));
        };

        return Ok((command, Some(PathBuf::from(path))));
    }

    if let Some(path) = config_equals_path(&first_arg) {
        let Some(command) = args.next() else {
            return Err(single_cli_error("missing command after `--config`"));
        };

        return Ok((command, Some(path)));
    }

    Ok((first_arg, None))
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
    config: Option<PathBuf>,
) -> core::DiagnosticResult<CommandOptions> {
    let mut options = CommandOptions {
        config,
        ..CommandOptions::default()
    };
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
        } else if let Some(path) = config_equals_path(&arg) {
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

fn config_equals_path(arg: &OsString) -> Option<PathBuf> {
    arg.to_str()
        .and_then(|arg| arg.strip_prefix("--config="))
        .map(PathBuf::from)
}

fn config_before_unsupported_command() -> core::DiagnosticReport {
    single_cli_error("`--config` may only be used with `check` or `compile`")
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
            parse_args(["sqlay", "check"].map(OsString::from)).expect("args should parse"),
            Command::Check { config: None }
        );
    }

    #[test]
    fn parses_help_flag() {
        assert_eq!(
            parse_args(["sqlay", "--help"].map(OsString::from)).expect("args should parse"),
            Command::Help(HelpTopic::TopLevel)
        );
    }

    #[test]
    fn parses_no_args_as_top_level_help() {
        assert_eq!(
            parse_args(["sqlay"].map(OsString::from)).expect("args should parse"),
            Command::Help(HelpTopic::TopLevel)
        );
    }

    #[test]
    fn parses_command_help_flags() {
        for (args, expected) in [
            (["sqlay", "init", "--help"], HelpTopic::Init),
            (["sqlay", "check", "--help"], HelpTopic::Check),
            (["sqlay", "compile", "--help"], HelpTopic::Compile),
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
            parse_args(["sqlay", "init"].map(OsString::from)).expect("args should parse"),
            Command::Init
        );
    }

    #[test]
    fn parses_compile_with_config_path() {
        assert_eq!(
            parse_args(
                ["sqlay", "compile", "--config", "custom/sqlay.config.json",].map(OsString::from),
            )
            .expect("args should parse"),
            Command::Compile {
                config: Some(PathBuf::from("custom/sqlay.config.json")),
                clean: false,
            }
        );
    }

    #[test]
    fn parses_top_level_config_path_before_check_command() {
        assert_eq!(
            parse_args(
                ["sqlay", "--config", "custom/sqlay.config.json", "check",].map(OsString::from),
            )
            .expect("args should parse"),
            Command::Check {
                config: Some(PathBuf::from("custom/sqlay.config.json")),
            }
        );
    }

    #[test]
    fn parses_top_level_equals_form_config_path_before_compile_command() {
        assert_eq!(
            parse_args(
                [
                    "sqlay",
                    "--config=custom/sqlay.config.json",
                    "compile",
                    "--clean",
                ]
                .map(OsString::from),
            )
            .expect("args should parse"),
            Command::Compile {
                config: Some(PathBuf::from("custom/sqlay.config.json")),
                clean: true,
            }
        );
    }

    #[test]
    fn parses_compile_clean() {
        assert_eq!(
            parse_args(["sqlay", "compile", "--clean"].map(OsString::from))
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
                ["sqlay", "check", "--config=custom/sqlay.config.json"].map(OsString::from),
            )
            .expect("args should parse"),
            Command::Check {
                config: Some(PathBuf::from("custom/sqlay.config.json")),
            }
        );
    }

    #[test]
    fn rejects_missing_config_path() {
        let report = parse_args(["sqlay", "check", "--config"].map(OsString::from))
            .expect_err("missing config value should fail");

        assert_eq!(
            report.diagnostics()[0].message(),
            "missing value for `--config`"
        );
    }
}
