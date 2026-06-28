use std::process::Command;

#[test]
fn no_args_prints_top_level_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .output()
        .expect("sqlay binary should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(
        stdout.contains("sqlay <command> [options]"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("sqlay init"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay check"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay compile"), "stdout: {stdout}");
    assert!(stdout.contains("/* @sqlay"), "stdout: {stdout}");
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
    assert!(stdout.contains("--fail-on-empty"), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "Place sqlay.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Query metadata:"), "stdout: {stdout}");
    assert!(
        stdout.contains("use paired @sqlay Param markers around a sample expression"),
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
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("--help")
        .output()
        .expect("sqlay help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sqlay init"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay check"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay compile"), "stdout: {stdout}");
}

#[test]
fn init_help_describes_init_behavior() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["init", "--help"])
        .output()
        .expect("sqlay init help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay init"), "stdout: {stdout}");
    assert!(
        stdout.contains("starter sqlay.config.json"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("refuses to overwrite"), "stdout: {stdout}");
}

#[test]
fn check_help_describes_config_discovery_and_database_url() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["check", "--help"])
        .output()
        .expect("sqlay check help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay check"), "stdout: {stdout}");
    assert!(stdout.contains("--config <path>"), "stdout: {stdout}");
    assert!(stdout.contains("--fail-on-empty"), "stdout: {stdout}");
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
            "Place sqlay.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Fragment, Slot, Repeat, validation case counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("compiled builders with query and mutation counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("per-query/per-mutation Param, Slot, Repeat, and validation case counts"),
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
    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--help"])
        .output()
        .expect("sqlay compile help should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("sqlay compile"), "stdout: {stdout}");
    assert!(stdout.contains("--config <path>"), "stdout: {stdout}");
    assert!(stdout.contains("--fail-on-empty"), "stdout: {stdout}");
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
            "Place sqlay.config.json at the project root when SQL lives in sibling directories"
        ),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Fragment, Slot, Repeat, validation case counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("compiled builders with query and mutation counts"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("per-query/per-mutation Param, Slot, Repeat, and validation case counts"),
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
