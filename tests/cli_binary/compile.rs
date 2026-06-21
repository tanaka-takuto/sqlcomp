use std::process::Command;

use crate::support::{
    TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL, VALID_CONFIG, unique_temp_dir,
    write_fragment_only_project, write_managed_generated_file,
};

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
