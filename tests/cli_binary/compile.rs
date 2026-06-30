use std::process::Command;

use crate::support::{
    TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL, VALID_CONFIG, assert_empty_source_diagnostic,
    unique_temp_dir, write_fragment_only_project, write_managed_generated_file,
    write_simple_query_project,
};

#[test]
fn compile_prints_generated_or_updated_file_count() {
    let config_dir = unique_temp_dir("sqlay-cli-compile-success-summary");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("notes.sql"), "-- comment only\n")
        .expect("comment-only SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Compile succeeded."), "stdout: {stdout}");
    assert!(stdout.contains("Matched 1 SQL file."), "stdout: {stdout}");
    assert!(
        stdout.contains("Compiled 0 builders: 0 queries, 0 mutations."),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Resolved 0 fragments."), "stdout: {stdout}");
    assert!(
        stdout.contains("Resolved 0 unique slots."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Resolved 0 unique repeats."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Validated 0 validation cases."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "Output dir: {}",
            std::fs::canonicalize(&config_dir)
                .expect("config dir should canonicalize")
                .join("src/generated/sqlay")
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

#[test]
fn compile_warns_when_source_include_matches_no_sql_files() {
    let config_dir = unique_temp_dir("sqlay-cli-empty-source-compile-warning");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning:"), "stderr: {stderr}");
    assert_empty_source_diagnostic(&stderr, &config_dir);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Matched 0 SQL files."), "stdout: {stdout}");
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_clean_skips_stale_cleanup_when_source_include_matches_no_sql_files() {
    let config_dir = unique_temp_dir("sqlay-cli-empty-source-clean-skip");
    let stale_path = config_dir.join("src/generated/sqlay/sql/stale.ts");
    std::fs::create_dir_all(
        stale_path
            .parent()
            .expect("stale file should have a parent"),
    )
    .expect("temp output dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile --clean should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning:"), "stderr: {stderr}");
    assert_empty_source_diagnostic(&stderr, &config_dir);
    assert!(
        stderr.contains("skipped stale generated file cleanup because no SQL files matched"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("pass `--allow-empty-clean` with `--clean`"),
        "stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Matched 0 SQL files."), "stdout: {stdout}");
    assert!(
        !stdout.contains("Removed 1 stale generated file."),
        "stdout: {stdout}"
    );
    assert!(
        stale_path.exists(),
        "compile --clean should skip stale cleanup when no SQL files matched"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_clean_allow_empty_clean_removes_stale_files_for_empty_source_matches() {
    let config_dir = unique_temp_dir("sqlay-cli-empty-source-clean-allow");
    let stale_path = config_dir.join("src/generated/sqlay/sql/stale.ts");
    std::fs::create_dir_all(
        stale_path
            .parent()
            .expect("stale file should have a parent"),
    )
    .expect("temp output dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--clean", "--allow-empty-clean"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile --clean should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning:"), "stderr: {stderr}");
    assert_empty_source_diagnostic(&stderr, &config_dir);
    assert!(
        !stderr.contains("skipped stale generated file cleanup"),
        "stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Matched 0 SQL files."), "stdout: {stdout}");
    assert!(
        stdout.contains("Removed 1 stale generated file."),
        "stdout: {stdout}"
    );
    assert!(
        !stale_path.exists(),
        "compile --clean --allow-empty-clean should clean stale generated files"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_fail_on_empty_rejects_empty_source_matches_before_cleaning() {
    let config_dir = unique_temp_dir("sqlay-cli-empty-source-compile-fail");
    let stale_path = config_dir.join("src/generated/sqlay/sql/stale.ts");
    std::fs::create_dir_all(
        stale_path
            .parent()
            .expect("stale file should have a parent"),
    )
    .expect("temp output dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--clean", "--fail-on-empty"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile should run");

    assert_eq!(output.status.code(), Some(1), "status: {:?}", output.status);
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"), "stderr: {stderr}");
    assert_empty_source_diagnostic(&stderr, &config_dir);
    assert!(
        stderr.contains("disable `--fail-on-empty`"),
        "stderr: {stderr}"
    );
    assert!(
        stale_path.exists(),
        "compile --fail-on-empty should stop before cleaning stale generated files"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_fail_on_empty_reports_empty_source_before_database_url_requirement() {
    let config_dir = unique_temp_dir("sqlay-cli-empty-source-compile-fail-without-database-url");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--fail-on-empty"])
        .current_dir(&config_dir)
        .env_remove(TEST_DATABASE_URL_ENV)
        .output()
        .expect("sqlay compile should run");

    assert_eq!(output.status.code(), Some(1), "status: {:?}", output.status);
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"), "stderr: {stderr}");
    assert_empty_source_diagnostic(&stderr, &config_dir);
    assert!(
        !stderr.contains("database.urlEnv"),
        "stderr should not require the database URL before empty-source enforcement: {stderr}"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

/// Verifies fragment-only SQL sources do not create path-parity `.ts` files.

#[test]
fn compile_does_not_create_fragment_only_output_files() {
    let config_dir = unique_temp_dir("sqlay-cli-fragment-only-compile");
    write_fragment_only_project(&config_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Matched 1 SQL file."), "stdout: {stdout}");
    assert!(
        stdout.contains("Compiled 0 builders: 0 queries, 0 mutations."),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("Resolved 1 fragment."), "stdout: {stdout}");
    assert!(
        stdout.contains("Generated or updated 0 files."),
        "stdout: {stdout}"
    );
    assert!(
        !config_dir
            .join("src/generated/sqlay/sql/fragments.ts")
            .exists(),
        "fragment-only SQL files must not generate TypeScript output"
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

/// Verifies normal compile still does not clean stale managed outputs.

#[test]
fn compile_keeps_stale_fragment_only_output_without_clean() {
    let config_dir = unique_temp_dir("sqlay-cli-fragment-only-stale-compile");
    let output_dir = write_fragment_only_project(&config_dir);
    let stale_path = output_dir.join("fragments.ts");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("compile")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile should run");

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
    let config_dir = unique_temp_dir("sqlay-cli-fragment-only-clean");
    let output_dir = write_fragment_only_project(&config_dir);
    let stale_path = output_dir.join("fragments.ts");
    write_managed_generated_file(&stale_path);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile --clean should run");

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
    let config_dir = unique_temp_dir("sqlay-cli-compile-clean-success-summary");
    let output_dir = config_dir.join("src/generated/sqlay/sql");
    std::fs::create_dir_all(&output_dir).expect("temp output dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    let stale_path = output_dir.join("old_users.ts");
    std::fs::write(
        &stale_path,
        "// @generated by sqlay. Do not edit.\nexport {}\n",
    )
    .expect("stale generated file should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--clean", "--allow-empty-clean"])
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay compile --clean should run");

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
    let config_dir = unique_temp_dir("sqlay-cli-compile-clean");
    write_simple_query_project(&config_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .env_remove(TEST_DATABASE_URL_ENV)
        .output()
        .expect("sqlay compile should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(
            "environment variable `SQLAY_TEST_DATABASE_URL` configured by `database.urlEnv` is not set"
        ),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}
