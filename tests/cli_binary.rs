use std::process::Command;

const TEST_DATABASE_URL_ENV: &str = "SQLCOMP_TEST_DATABASE_URL";
const UNUSED_DATABASE_URL: &str = "mysql://sqlcomp:sqlcomp@127.0.0.1:3306/sqlcomp";
const DUPLICATE_IDS_FIXTURE: &str = include_str!("../fixtures/sql/invalid/duplicate_ids.sql");
const EXEC_CARDINALITY_FIXTURE: &str = include_str!("../fixtures/sql/invalid/exec_cardinality.sql");

const VALID_CONFIG: &str = r#"
{
  "source": {
    "include": ["sql/**/*.sql"]
  },
  "output": {
    "dir": "src/generated/sqlcomp"
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "SQLCOMP_TEST_DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
"#;

const UNSUPPORTED_CONFIG: &str = r#"
{
  "source": {
    "include": ["sql/**/*.sql"]
  },
  "output": {
    "dir": "src/generated/sqlcomp"
  },
  "database": {
    "dialect": "postgres",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "go"
  }
}
"#;

const INVALID_SOURCE_CONFIG: &str = r#"
{
  "source": {
    "include": ["invalid/**/*.sql"]
  },
  "output": {
    "dir": "generated-invalid"
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "SQLCOMP_TEST_DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
"#;

const FRAGMENT_ONLY_SQL: &str = r"
/* @sqlcomp
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1
";

const NESTED_CONFIG_WITH_PARENT_INCLUDE: &str = r#"
{
  "source": {
    "include": ["../sql/**/*.sql"]
  },
  "output": {
    "dir": "generated"
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "SQLCOMP_TEST_DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
"#;

#[test]
fn no_args_prints_top_level_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .output()
        .expect("sqlcomp binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(
        stdout.contains("sqlcomp <command> [options]"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("sqlcomp init"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp check"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp compile"), "stdout: {stdout}");
    assert!(stdout.contains("/* @sqlcomp"), "stdout: {stdout}");
    assert!(stdout.contains("type: query"), "stdout: {stdout}");
    assert!(stdout.contains("id: listUsers"), "stdout: {stdout}");
    assert!(
        stdout.contains("cardinality: one | many"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("ordinary SQL comments"), "stdout: {stdout}");
    assert!(stdout.contains("raw `?` placeholders"), "stdout: {stdout}");
    assert!(
        stdout.contains("source.include paths must stay inside the config directory"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Place sqlcomp.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Query metadata:"), "stdout: {stdout}");
    assert!(
        stdout.contains("use paired @sqlcomp Param markers around a sample expression"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("type: param id: emailFilter valueType: string nullable: true"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("id: listCustomersByFilter"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("type: param id: createdBefore valueType: datetime nullable: true"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("createdBefore: string | null;"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Use nullable: true for T | null inputs; optional input properties are not supported"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Repeat the same Param id for optional filters; params follow marker occurrence order"
        ),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("MVP query metadata"),
        "stdout should not describe current help as MVP-only: {stdout}"
    );
    assert!(
        !stdout.contains("when dynamic inputs are supported"),
        "stdout should not describe Param markers as future-only: {stdout}"
    );
}

#[test]
fn help_lists_supported_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("--help")
        .output()
        .expect("sqlcomp help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sqlcomp init"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp check"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp compile"), "stdout: {stdout}");
}

#[test]
fn init_help_describes_init_behavior() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["init", "--help"])
        .output()
        .expect("sqlcomp init help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp init"), "stdout: {stdout}");
    assert!(
        stdout.contains("starter sqlcomp.config.json"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("refuses to overwrite"), "stdout: {stdout}");
}

#[test]
fn check_help_describes_config_discovery_and_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["check", "--help"])
        .output()
        .expect("sqlcomp check help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp check"), "stdout: {stdout}");
    assert!(stdout.contains("--config <path>"), "stdout: {stdout}");
    assert!(
        stdout.contains("searches from the current working directory upward"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("database.urlEnv"), "stdout: {stdout}");
    assert!(stdout.contains("No files are written"), "stdout: {stdout}");
    assert!(
        stdout.contains("preserves each input SQL path"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("source.include paths must stay inside the config directory"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Place sqlcomp.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Fragment, Slot, variant counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("per-query Param, Slot, and variant counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("type: param id: emailFilter valueType: string nullable: true"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("id: listCustomersByFilter"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("type: param id: createdBefore valueType: datetime nullable: true"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("createdBefore: string | null;"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Use nullable: true for T | null inputs; optional input properties are not supported"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Repeat the same Param id for optional filters; params follow marker occurrence order"
        ),
        "stdout: {stdout}"
    );
}

#[test]
fn compile_help_describes_output_writing_and_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["compile", "--help"])
        .output()
        .expect("sqlcomp compile help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlcomp compile"), "stdout: {stdout}");
    assert!(stdout.contains("--config <path>"), "stdout: {stdout}");
    assert!(
        stdout.contains("generated TypeScript files"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("--clean"), "stdout: {stdout}");
    assert!(stdout.contains("stale generated files"), "stdout: {stdout}");
    assert!(
        stdout.contains("preserves each input SQL path"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("source.include paths must stay inside the config directory"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Place sqlcomp.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Fragment, Slot, variant counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("per-query Param, Slot, and variant counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("type: param id: emailFilter valueType: string nullable: true"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("id: listCustomersByFilter"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("type: param id: createdBefore valueType: datetime nullable: true"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("createdBefore: string | null;"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("valueType values: bool, int32, int64, float64, decimal, string, bytes, date, time, datetime, json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Use nullable: true for T | null inputs; optional input properties are not supported"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "Repeat the same Param id for optional filters; params follow marker occurrence order"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("BIGINT, DECIMAL, date/time, and enum values map conservatively"),
        "stdout: {stdout}"
    );
}

#[test]
fn check_discovers_config_from_current_directory() {
    let config_dir = unique_temp_dir("sqlcomp-cli-discovery-root");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_discovers_config_from_nested_child_directory() {
    let config_dir = unique_temp_dir("sqlcomp-cli-discovery-nested");
    let child_dir = config_dir.join("packages").join("api").join("sql");
    std::fs::create_dir_all(&child_dir).expect("temp child dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&child_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_explains_config_relative_source_include_boundary() {
    let project_dir = unique_temp_dir("sqlcomp-cli-source-boundary");
    let configs_dir = project_dir.join("configs");
    let sql_dir = project_dir.join("sql").join("qa");
    std::fs::create_dir_all(&configs_dir).expect("temp config dir should be created");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    let config_path = configs_dir.join("sqlcomp.qa.json");
    let sql_path = sql_dir.join("order_nullable_probe.sql");
    std::fs::write(&config_path, NESTED_CONFIG_WITH_PARENT_INCLUDE)
        .expect("nested config should be written");
    std::fs::write(
        &sql_path,
        r"
/* @sqlcomp
{
  type: query
  id: orderNullableProbe
}
*/
SELECT 1;
"
        .strip_prefix('\n')
        .expect("raw SQL fixture should start with a newline"),
    )
    .expect("temp SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["check", "--config"])
        .arg(&config_path)
        .current_dir(&project_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("source file"), "stderr: {stderr}");
    assert!(
        stderr.contains("is outside the configuration directory"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("source.include paths are resolved from the config file directory and must stay inside it"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("Move sqlcomp.config.json to a common project root when SQL lives in sibling directories"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(
            "generated paths can be preserved relative to that directory under output.dir"
        ),
        "stderr: {stderr}"
    );

    std::fs::remove_dir_all(project_dir).expect("temp config tree should be removed");
}

#[test]
fn check_warns_for_included_unannotated_sql_file() {
    let config_dir = unique_temp_dir("sqlcomp-cli-unannotated-sql-warning");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    let sql_path = sql_dir.join("users.sql");
    std::fs::write(&sql_path, "SELECT id FROM users;\n").expect("temp SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&sql_path.display().to_string()),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("warning:"), "stderr: {stderr}");
    assert!(stderr.contains("@sqlcomp"), "stderr: {stderr}");
    assert!(stderr.contains("type: query"), "stderr: {stderr}");

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_does_not_warn_for_empty_or_comment_only_sql_files() {
    let config_dir = unique_temp_dir("sqlcomp-cli-comment-only-sql");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("empty.sql"), "\n\n").expect("empty SQL should be written");
    std::fs::write(
        sql_dir.join("comments.sql"),
        "-- comment only\n# another comment\n/* block comment */\n",
    )
    .expect("comment-only SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_prints_success_summary_without_implying_writes() {
    let config_dir = unique_temp_dir("sqlcomp-cli-check-success-summary");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("notes.sql"), "-- comment only\n")
        .expect("comment-only SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check passed."), "stdout: {stdout}");
    assert!(stdout.contains("Matched 1 SQL file."), "stdout: {stdout}");
    assert!(stdout.contains("Compiled 0 queries."), "stdout: {stdout}");
    assert!(stdout.contains("Resolved 0 fragments."), "stdout: {stdout}");
    assert!(
        stdout.contains("Resolved 0 unique slots."),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Validated 0 variants."), "stdout: {stdout}");
    assert!(
        stdout.contains(&format!(
            "Output dir: {}",
            std::fs::canonicalize(&config_dir)
                .expect("config dir should canonicalize")
                .join("src/generated/sqlcomp")
                .display()
        )),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("No files written."), "stdout: {stdout}");
    assert!(stdout.contains("Queries: none."), "stdout: {stdout}");

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_prints_generated_or_updated_file_count() {
    let config_dir = unique_temp_dir("sqlcomp-cli-compile-success-summary");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("notes.sql"), "-- comment only\n")
        .expect("comment-only SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp compile should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Compile succeeded."), "stdout: {stdout}");
    assert!(stdout.contains("Matched 1 SQL file."), "stdout: {stdout}");
    assert!(stdout.contains("Compiled 0 queries."), "stdout: {stdout}");
    assert!(stdout.contains("Resolved 0 fragments."), "stdout: {stdout}");
    assert!(
        stdout.contains("Resolved 0 unique slots."),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Validated 0 variants."), "stdout: {stdout}");
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "Output dir: {}",
            std::fs::canonicalize(&config_dir)
                .expect("config dir should canonicalize")
                .join("src/generated/sqlcomp")
                .display()
        )),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Generated files: none."),
        "stdout: {stdout}"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

/// Verifies fragment-only SQL sources do not create path-parity `.ts` files.
#[test]
fn compile_does_not_create_fragment_only_output_files() {
    let config_dir = unique_temp_dir("sqlcomp-cli-fragment-only-compile");
    write_fragment_only_project(&config_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp compile should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Matched 1 SQL file."), "stdout: {stdout}");
    assert!(stdout.contains("Compiled 0 queries."), "stdout: {stdout}");
    assert!(stdout.contains("Resolved 1 fragment."), "stdout: {stdout}");
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );
    assert!(
        !config_dir
            .join("src/generated/sqlcomp/sql/fragments.ts")
            .exists(),
        "fragment-only SQL files must not generate TypeScript output"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

/// Verifies normal compile still does not clean stale managed outputs.
#[test]
fn compile_keeps_stale_fragment_only_output_without_clean() {
    let config_dir = unique_temp_dir("sqlcomp-cli-fragment-only-stale-compile");
    let output_dir = write_fragment_only_project(&config_dir);
    let stale_path = output_dir.join("fragments.ts");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp compile should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stale_path.exists(),
        "normal compile must leave stale generated files untouched"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

/// Verifies compile --clean removes a stale output for a now fragment-only source.
#[test]
fn compile_clean_removes_stale_fragment_only_output() {
    let config_dir = unique_temp_dir("sqlcomp-cli-fragment-only-clean");
    let output_dir = write_fragment_only_project(&config_dir);
    let stale_path = output_dir.join("fragments.ts");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp compile --clean should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Removed 1 stale generated file."),
        "stdout: {stdout}"
    );
    assert!(
        !stale_path.exists(),
        "compile --clean should remove stale generated output for fragment-only SQL"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_clean_prints_removed_stale_generated_file_count() {
    let config_dir = unique_temp_dir("sqlcomp-cli-compile-clean-success-summary");
    let output_dir = config_dir.join("src/generated/sqlcomp/sql");
    std::fs::create_dir_all(&output_dir).expect("temp output dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    let stale_path = output_dir.join("old_users.ts");
    std::fs::write(
        &stale_path,
        "// @generated by sqlcomp. Do not edit.\nexport {}\n",
    )
    .expect("stale generated file should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp compile --clean should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Compile succeeded."), "stdout: {stdout}");
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Removed 1 stale generated file."),
        "stdout: {stdout}"
    );
    assert!(
        !stale_path.exists(),
        "compile --clean should remove stale generated files"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn explicit_config_path_bypasses_upward_discovery() {
    let config_dir = unique_temp_dir("sqlcomp-cli-explicit-config");
    let child_dir = config_dir.join("packages").join("api");
    let explicit_path = child_dir.join("sqlcomp.config.json");
    std::fs::create_dir_all(&child_dir).expect("temp child dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("parent config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["check", "--config"])
        .arg(&explicit_path)
        .current_dir(&child_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!(
            "failed to read config file `{}`",
            explicit_path.display()
        )),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn top_level_config_path_is_accepted_before_check_command() {
    let config_dir = unique_temp_dir("sqlcomp-cli-top-level-config");
    let child_dir = config_dir.join("packages").join("api");
    let explicit_path = child_dir.join("sqlcomp.config.json");
    std::fs::create_dir_all(&child_dir).expect("temp child dir should be created");
    std::fs::write(&explicit_path, VALID_CONFIG).expect("explicit config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["--config"])
        .arg(&explicit_path)
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check passed."), "stdout: {stdout}");

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_reports_when_config_is_not_found() {
    let start_dir = unique_temp_dir("sqlcomp-cli-missing-config");
    std::fs::create_dir_all(&start_dir).expect("temp start dir should be created");
    let canonical_start_dir =
        std::fs::canonicalize(&start_dir).expect("temp start dir should canonicalize");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&start_dir)
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!(
            "failed to find `sqlcomp.config.json` from `{}` or any parent directory",
            canonical_start_dir.display()
        )),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(start_dir).expect("temp start dir should be removed");
}

#[test]
fn check_reports_unsupported_config_before_pipeline_skeleton() {
    let config_dir = unique_temp_dir("sqlcomp-cli-unsupported-config");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), UNSUPPORTED_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "unsupported config field `database.dialect` value `postgres`; supported value is `mysql`"
        ),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(
            "unsupported config field `target.language` value `go`; supported value is `typescript`"
        ),
        "stderr: {stderr}"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_reports_missing_database_url_environment_variable() {
    let config_dir = unique_temp_dir("sqlcomp-cli-missing-database-url");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env_remove(TEST_DATABASE_URL_ENV)
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "environment variable `SQLCOMP_TEST_DATABASE_URL` configured by `database.urlEnv` is not set"
        ),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_reports_multiple_source_intake_diagnostics_in_one_run() {
    let config_dir = unique_temp_dir("sqlcomp-cli-multiple-source-diagnostics");
    let invalid_dir = config_dir.join("invalid");
    std::fs::create_dir_all(&invalid_dir).expect("temp invalid SQL dir should be created");
    std::fs::write(
        config_dir.join("sqlcomp.config.json"),
        INVALID_SOURCE_CONFIG,
    )
    .expect("temp config should be written");
    std::fs::write(invalid_dir.join("duplicate_ids.sql"), DUPLICATE_IDS_FIXTURE)
        .expect("duplicate id fixture should be written");
    std::fs::write(
        invalid_dir.join("exec_cardinality.sql"),
        EXEC_CARDINALITY_FIXTURE,
    )
    .expect("exec cardinality fixture should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicate query id `duplicatedQuery`"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("first declared here"), "stderr: {stderr}");
    assert!(
        stderr.contains("`cardinality: exec` is reserved for future non-SELECT support"),
        "stderr: {stderr}"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn init_writes_starter_config_to_current_directory() {
    let config_dir = unique_temp_dir("sqlcomp-cli-init");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("init")
        .current_dir(&config_dir)
        .output()
        .expect("sqlcomp init should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Created sqlcomp.config.json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("DATABASE_URL=... sqlcomp check"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("/* @sqlcomp"), "stdout: {stdout}");
    assert!(stdout.contains("type: query"), "stdout: {stdout}");
    assert!(stdout.contains("id: listUsers"), "stdout: {stdout}");
    assert!(
        stdout.contains("SELECT id, name FROM users;"),
        "stdout: {stdout}"
    );

    let config_path = config_dir.join("sqlcomp.config.json");
    let config = std::fs::read_to_string(&config_path).expect("starter config should be written");
    assert!(
        config.contains(r#""include": ["sql/**/*.sql"]"#),
        "config: {config}"
    );
    assert!(config.contains(r#""exclude": []"#), "config: {config}");
    assert!(
        config.contains(r#""dir": "src/generated/sqlcomp""#),
        "config: {config}"
    );
    assert!(config.contains(r#""dialect": "mysql""#), "config: {config}");
    assert!(
        config.contains(r#""urlEnv": "DATABASE_URL""#),
        "config: {config}"
    );
    assert!(
        config.contains(r#""language": "typescript""#),
        "config: {config}"
    );

    let check_output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("check")
        .current_dir(&config_dir)
        .env("DATABASE_URL", UNUSED_DATABASE_URL)
        .output()
        .expect("sqlcomp check should run");

    assert!(
        check_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn init_refuses_to_overwrite_existing_config() {
    let config_dir = unique_temp_dir("sqlcomp-cli-init-existing");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    let config_path = config_dir.join("sqlcomp.config.json");
    std::fs::write(&config_path, "keep me").expect("existing config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .arg("init")
        .current_dir(&config_dir)
        .output()
        .expect("sqlcomp init should run");

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(&config_path).expect("existing config should still exist"),
        "keep me"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("refusing to overwrite existing config file"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_clean_uses_compile_pipeline_database_configuration() {
    let config_dir = unique_temp_dir("sqlcomp-cli-compile-clean");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .env_remove(TEST_DATABASE_URL_ENV)
        .output()
        .expect("sqlcomp compile should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "environment variable `SQLCOMP_TEST_DATABASE_URL` configured by `database.urlEnv` is not set"
        ),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
}

/// Writes a config plus one fragment-only SQL source and returns the output SQL dir.
fn write_fragment_only_project(config_dir: &std::path::Path) -> std::path::PathBuf {
    let sql_dir = config_dir.join("sql");
    let output_dir = config_dir.join("src/generated/sqlcomp/sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::create_dir_all(&output_dir).expect("temp output dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("fragments.sql"), FRAGMENT_ONLY_SQL)
        .expect("fragment-only SQL should be written");

    output_dir
}

/// Writes a managed generated file fixture that clean can classify as stale.
fn write_managed_generated_file(path: &std::path::Path) {
    std::fs::write(path, "// @generated by sqlcomp. Do not edit.\nexport {}\n")
        .expect("stale generated fragment output should be written");
}
