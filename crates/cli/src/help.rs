pub const HELP: &str = "\
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

pub const INIT_HELP: &str = "\
Create a starter sqlcomp.config.json.

Usage:
  sqlcomp init

Behavior:
  Writes a starter sqlcomp.config.json in the current directory and refuses to overwrite an existing config file.

Examples:
  sqlcomp init
";

pub const CHECK_HELP: &str = "\
Check SQL sources without writing generated files.

Usage:
  sqlcomp check [options]

Behavior:
  Loads sqlcomp.config.json, reads SQL files, validates MySQL SELECT queries, and renders generated TypeScript output in memory.
  When --config is omitted, searches from the current working directory upward for sqlcomp.config.json.
  Reads the database URL from the environment variable named by database.urlEnv.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path.

Examples:
  DATABASE_URL=... sqlcomp check
  sqlcomp check --config ./sqlcomp.config.json
";

pub const COMPILE_HELP: &str = "\
Compile SQL sources to generated TypeScript files.

Usage:
  sqlcomp compile [options]

Behavior:
  Loads sqlcomp.config.json, validates SQL sources, and writes generated TypeScript files under output.dir.
  When --config is omitted, searches from the current working directory upward for sqlcomp.config.json.
  Reads the database URL from the environment variable named by database.urlEnv.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path.
  --clean            Remove stale generated files that no longer correspond to input SQL files.

Examples:
  DATABASE_URL=... sqlcomp compile
  sqlcomp compile --config ./sqlcomp.config.json --clean
";

pub const INIT_NEXT_STEPS: &str = r"
Next:
  DATABASE_URL=... sqlcomp check

Add a query block such as:
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id, name FROM users;
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpTopic {
    TopLevel,
    Init,
    Check,
    Compile,
}

pub const fn help_text(topic: HelpTopic) -> &'static str {
    match topic {
        HelpTopic::TopLevel => HELP,
        HelpTopic::Init => INIT_HELP,
        HelpTopic::Check => CHECK_HELP,
        HelpTopic::Compile => COMPILE_HELP,
    }
}
