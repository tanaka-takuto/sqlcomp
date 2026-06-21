use std::process::Command;

use crate::support::{UNUSED_DATABASE_URL, unique_temp_dir};

#[test]
fn init_writes_starter_config_to_current_directory() {
    let config_dir = unique_temp_dir("sqlay-cli-init");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("init")
        .current_dir(&config_dir)
        .output()
        .expect("sqlay init should run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Created sqlay.config.json"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("DATABASE_URL=... sqlay check"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("/* @sqlay"), "stdout: {stdout}");
    assert!(stdout.contains("type: query"), "stdout: {stdout}");
    assert!(stdout.contains("id: listUsers"), "stdout: {stdout}");
    assert!(
        stdout.contains("SELECT id, name FROM users;"),
        "stdout: {stdout}"
    );

    let config_path = config_dir.join("sqlay.config.json");
    let config = std::fs::read_to_string(&config_path).expect("starter config should be written");
    assert!(
        config.contains(r#""include": ["sql/**/*.sql"]"#),
        "config: {config}"
    );
    assert!(config.contains(r#""exclude": []"#), "config: {config}");
    assert!(
        config.contains(r#""dir": "src/generated/sqlay""#),
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

    let check_output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("check")
        .current_dir(&config_dir)
        .env("DATABASE_URL", UNUSED_DATABASE_URL)
        .output()
        .expect("sqlay check should run");

    assert!(
        check_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn init_refuses_to_overwrite_existing_config() {
    let config_dir = unique_temp_dir("sqlay-cli-init-existing");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    let config_path = config_dir.join("sqlay.config.json");
    std::fs::write(&config_path, "keep me").expect("existing config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlay"))
        .arg("init")
        .current_dir(&config_dir)
        .output()
        .expect("sqlay init should run");

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
