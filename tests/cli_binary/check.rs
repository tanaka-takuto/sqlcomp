use std::process::Command;

use crate::support::{
    DUPLICATE_IDS_FIXTURE, EXEC_CARDINALITY_FIXTURE, INVALID_SOURCE_CONFIG, TEST_DATABASE_URL_ENV,
    UNSUPPORTED_CONFIG, UNUSED_DATABASE_URL, VALID_CONFIG, unique_temp_dir,
};

#[test]
fn check_warns_for_included_unannotated_sql_file() {
    let config_dir = unique_temp_dir("sqlay-cli-unannotated-sql-warning");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    let sql_path = sql_dir.join("users.sql");
    std::fs::write(&sql_path, "SELECT id FROM users;\n").expect("temp SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

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
    assert!(stderr.contains("@sqlay"), "stderr: {stderr}");
    assert!(stderr.contains("type: query"), "stderr: {stderr}");

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_does_not_warn_for_empty_or_comment_only_sql_files() {
    let config_dir = unique_temp_dir("sqlay-cli-comment-only-sql");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("empty.sql"), "\n\n").expect("empty SQL should be written");
    std::fs::write(
        sql_dir.join("comments.sql"),
        "-- comment only\n# another comment\n/* block comment */\n",
    )
    .expect("comment-only SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

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
    let config_dir = unique_temp_dir("sqlay-cli-check-success-summary");
    let sql_dir = config_dir.join("sql");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");
    std::fs::write(sql_dir.join("notes.sql"), "-- comment only\n")
        .expect("comment-only SQL should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

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
                .join("src/generated/sqlay")
                .display()
        )),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("No files written."), "stdout: {stdout}");
    assert!(stdout.contains("Queries: none."), "stdout: {stdout}");

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_reports_unsupported_config_before_pipeline_skeleton() {
    let config_dir = unique_temp_dir("sqlay-cli-unsupported-config");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), UNSUPPORTED_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .output()
        .expect("sqlay check should run");

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
    let config_dir = unique_temp_dir("sqlay-cli-missing-database-url");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .env_remove(TEST_DATABASE_URL_ENV)
        .output()
        .expect("sqlay check should run");

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

#[test]
fn check_reports_multiple_source_intake_diagnostics_in_one_run() {
    let config_dir = unique_temp_dir("sqlay-cli-multiple-source-diagnostics");
    let invalid_dir = config_dir.join("invalid");
    std::fs::create_dir_all(&invalid_dir).expect("temp invalid SQL dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), INVALID_SOURCE_CONFIG)
        .expect("temp config should be written");
    std::fs::write(invalid_dir.join("duplicate_ids.sql"), DUPLICATE_IDS_FIXTURE)
        .expect("duplicate id fixture should be written");
    std::fs::write(
        invalid_dir.join("exec_cardinality.sql"),
        EXEC_CARDINALITY_FIXTURE,
    )
    .expect("exec cardinality fixture should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

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
