pub const HELP: &str = "\
SQL Inlay.

Usage:
  sqlay <command> [options]

Commands:
  sqlay init       Create a starter sqlay.config.json.
  sqlay check      Load config and run the compile pipeline without writing generated files.
  sqlay compile    Load config and write generated TypeScript files.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path for check or compile.
  --clean            Remove stale generated files during compile.

Minimal query annotation:
  /* @sqlay
  {
    type: query
    id: listUsers
    // cardinality: one | many
  }
  */
  SELECT id, name FROM users;

Query metadata:
  type: query is required.
  id is required and must match ^[A-Za-z_][A-Za-z0-9_]*$.
  cardinality is optional: one or many. cardinality: exec is rejected.

Directive boundary:
  Compiler directives are @sqlay Hjson block comments.
  Similar ordinary SQL comments such as /* @param tenantKey */ are ignored as SQL comments.
  Do not write raw `?` placeholders in source SQL; use paired @sqlay Param markers around a sample expression.
  Slot and Fragment composition is available for optional single-select query-local slots.
  Repeat ranges are available for variable-length SQL repetition inside queries, mutations, and fragments.

Config path boundary:
  source.include paths must stay inside the config directory.
  Place sqlay.config.json at the project root when SQL lives in sibling directories.
  Generated TypeScript preserves each input SQL path relative to the config directory under output.dir.

Param marker example:
  /* @sqlay
  {
    type: query
    id: listCustomersByFilter
  }
  */
  SELECT customers.id, customers.email
  FROM customers
  WHERE (customers.email = /* @sqlay { type: param id: emailFilter valueType: string nullable: true } */
    'ada@example.test'
    /* @sqlay { type: paramEnd } */
    OR /* @sqlay { type: param id: emailFilter valueType: string nullable: true } */
    NULL
    /* @sqlay { type: paramEnd } */ IS NULL)
    AND (customers.created_at < /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
      '2026-01-01 00:00:00'
      /* @sqlay { type: paramEnd } */
      OR /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
      NULL
      /* @sqlay { type: paramEnd } */ IS NULL);

Generated TypeScript input:
  export type listCustomersByFilter_Input = {
    emailFilter: string | null;
    createdBefore: string | null;
  };

Param metadata:
  valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json.
  Use nullable: true for T | null inputs; optional input properties are not supported.
  Repeat the same Param id for optional filters; params follow marker occurrence order.
";

pub const INIT_HELP: &str = "\
Create a starter sqlay.config.json.

Usage:
  sqlay init

Behavior:
  Writes a starter sqlay.config.json in the current directory and refuses to overwrite an existing config file.
  Prints a minimal @sqlay query annotation and the next check command.

Examples:
  sqlay init
";

pub const CHECK_HELP: &str = "\
Check SQL sources without writing generated files.

Usage:
  sqlay check [options]

Behavior:
  Loads sqlay.config.json, reads SQL files, validates MySQL SELECT queries, and renders generated TypeScript output in memory.
  When --config is omitted, searches from the current working directory upward for sqlay.config.json.
  Reads the database URL from the environment variable named by database.urlEnv.
  No files are written.
  Generated TypeScript preserves each input SQL path relative to the config directory under output.dir.
  source.include paths must stay inside the config directory.
  Place sqlay.config.json at the project root when SQL lives in sibling directories.
  The success summary reports matched SQL files, compiled builders with query and mutation counts, Fragment, Slot, Repeat, validation case counts, output.dir, and per-query/per-mutation Param, Slot, Repeat, and validation case counts.

Param marker example:
  /* @sqlay
  {
    type: query
    id: listCustomersByFilter
  }
  */
  SELECT customers.id, customers.email
  FROM customers
  WHERE (customers.email = /* @sqlay { type: param id: emailFilter valueType: string nullable: true } */
    'ada@example.test'
    /* @sqlay { type: paramEnd } */
    OR /* @sqlay { type: param id: emailFilter valueType: string nullable: true } */
    NULL
    /* @sqlay { type: paramEnd } */ IS NULL)
    AND (customers.created_at < /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
      '2026-01-01 00:00:00'
      /* @sqlay { type: paramEnd } */
      OR /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
      NULL
      /* @sqlay { type: paramEnd } */ IS NULL);

Generated TypeScript input:
  export type listCustomersByFilter_Input = {
    emailFilter: string | null;
    createdBefore: string | null;
  };

Param metadata:
  valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json.
  Use nullable: true for T | null inputs; optional input properties are not supported.
  Repeat the same Param id for optional filters; params follow marker occurrence order.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path.

Examples:
  DATABASE_URL=... sqlay check
  sqlay check --config ./sqlay.config.json
";

pub const COMPILE_HELP: &str = "\
Compile SQL sources to generated TypeScript files.

Usage:
  sqlay compile [options]

Behavior:
  Loads sqlay.config.json, validates SQL sources, and writes generated TypeScript files under output.dir.
  When --config is omitted, searches from the current working directory upward for sqlay.config.json.
  Reads the database URL from the environment variable named by database.urlEnv.
  Generated TypeScript preserves each input SQL path relative to the config directory under output.dir.
  source.include paths must stay inside the config directory.
  Place sqlay.config.json at the project root when SQL lives in sibling directories.
  The success summary reports matched SQL files, compiled builders with query and mutation counts, Fragment, Slot, Repeat, validation case counts, generated file paths, stale-file cleanup, and per-query/per-mutation Param, Slot, Repeat, and validation case counts.
  TypeScript type mapping is conservative: BIGINT, DECIMAL, date/time, and enum values map conservatively to string; bytes map to Uint8Array; JSON and unknown types map to unknown; nullable metadata adds | null.

Param marker example:
  /* @sqlay
  {
    type: query
    id: listCustomersByFilter
  }
  */
  SELECT customers.id, customers.email
  FROM customers
  WHERE (customers.email = /* @sqlay { type: param id: emailFilter valueType: string nullable: true } */
    'ada@example.test'
    /* @sqlay { type: paramEnd } */
    OR /* @sqlay { type: param id: emailFilter valueType: string nullable: true } */
    NULL
    /* @sqlay { type: paramEnd } */ IS NULL)
    AND (customers.created_at < /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
      '2026-01-01 00:00:00'
      /* @sqlay { type: paramEnd } */
      OR /* @sqlay { type: param id: createdBefore valueType: datetime nullable: true } */
      NULL
      /* @sqlay { type: paramEnd } */ IS NULL);

Generated TypeScript input:
  export type listCustomersByFilter_Input = {
    emailFilter: string | null;
    createdBefore: string | null;
  };

Param metadata:
  valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json.
  Use nullable: true for T | null inputs; optional input properties are not supported.
  Repeat the same Param id for optional filters; params follow marker occurrence order.

Options:
  -h, --help         Print this help.
  --config <path>    Use an explicit config path.
  --clean            Remove stale generated files that no longer correspond to input SQL files.

Examples:
  DATABASE_URL=... sqlay compile
  sqlay compile --config ./sqlay.config.json --clean
";

pub const INIT_NEXT_STEPS: &str = r"
Next:
  DATABASE_URL=... sqlay check

Compiler directives are @sqlay Hjson block comments. Ordinary SQL comments such as
/* @param tenantKey */ are ignored as SQL comments.

Add a query block such as:
/* @sqlay
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
