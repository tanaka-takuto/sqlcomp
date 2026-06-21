use std::process::Command;

use crate::support::{
    NESTED_CONFIG_WITH_PARENT_INCLUDE, TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL, VALID_CONFIG,
    unique_temp_dir,
};

#[test]
fn check_discovers_config_from_current_directory() {
    let config_dir = unique_temp_dir("sqlay-cli-discovery-root");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

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

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_discovers_config_from_nested_child_directory() {
    let config_dir = unique_temp_dir("sqlay-cli-discovery-nested");
    let child_dir = config_dir.join("packages").join("api").join("sql");
    std::fs::create_dir_all(&child_dir).expect("temp child dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&child_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_explains_config_relative_source_include_boundary() {
    let project_dir = unique_temp_dir("sqlay-cli-source-boundary");
    let configs_dir = project_dir.join("configs");
    let sql_dir = project_dir.join("sql").join("qa");
    std::fs::create_dir_all(&configs_dir).expect("temp config dir should be created");
    std::fs::create_dir_all(&sql_dir).expect("temp SQL dir should be created");
    let config_path = configs_dir.join("sqlay.qa.json");
    let sql_path = sql_dir.join("order_nullable_probe.sql");
    std::fs::write(&config_path, NESTED_CONFIG_WITH_PARENT_INCLUDE)
        .expect("nested config should be written");
    std::fs::write(
        &sql_path,
        r"
/* @sqlay
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

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["check", "--config"])
        .arg(&config_path)
        .current_dir(&project_dir)
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
        stderr.contains(
            "Move sqlay.config.json to a common project root when SQL lives in sibling directories"
        ),
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
fn explicit_config_path_bypasses_upward_discovery() {
    let config_dir = unique_temp_dir("sqlay-cli-explicit-config");
    let child_dir = config_dir.join("packages").join("api");
    let explicit_path = child_dir.join("sqlay.config.json");
    std::fs::create_dir_all(&child_dir).expect("temp child dir should be created");
    std::fs::write(config_dir.join("sqlay.config.json"), VALID_CONFIG)
        .expect("parent config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["check", "--config"])
        .arg(&explicit_path)
        .current_dir(&child_dir)
        .env(TEST_DATABASE_URL_ENV, UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

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
    let config_dir = unique_temp_dir("sqlay-cli-top-level-config");
    let child_dir = config_dir.join("packages").join("api");
    let explicit_path = child_dir.join("sqlay.config.json");
    std::fs::create_dir_all(&child_dir).expect("temp child dir should be created");
    std::fs::write(&explicit_path, VALID_CONFIG).expect("explicit config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .args(["--config"])
        .arg(&explicit_path)
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

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn check_reports_when_config_is_not_found() {
    let start_dir = unique_temp_dir("sqlay-cli-missing-config");
    std::fs::create_dir_all(&start_dir).expect("temp start dir should be created");
    let canonical_start_dir =
        std::fs::canonicalize(&start_dir).expect("temp start dir should canonicalize");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&start_dir)
        .output()
        .expect("sqlay check should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!(
            "failed to find `sqlay.config.json` from `{}` or any parent directory",
            canonical_start_dir.display()
        )),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(start_dir).expect("temp start dir should be removed");
}
