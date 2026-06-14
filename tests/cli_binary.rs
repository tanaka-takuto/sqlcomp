use std::process::Command;

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
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "typescript"
  }
}
"#;

#[test]
fn sqlcomp_binary_exits_successfully() {
    let status = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .status()
        .expect("sqlcomp binary should run");

    assert!(status.success());
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
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("command `check` is not implemented yet"),
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
        .output()
        .expect("sqlcomp check should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("command `check` is not implemented yet"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
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
        .output()
        .expect("sqlcomp check should run");

    assert!(!check_output.status.success());
    assert!(
        String::from_utf8_lossy(&check_output.stderr)
            .contains("command `check` is not implemented yet"),
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

    std::fs::remove_dir_all(config_dir).expect("temp config tree should be removed");
}

#[test]
fn compile_clean_is_recognized_but_not_implemented_yet() {
    let config_dir = unique_temp_dir("sqlcomp-cli-compile-clean");
    std::fs::create_dir_all(&config_dir).expect("temp config dir should be created");
    std::fs::write(config_dir.join("sqlcomp.config.json"), VALID_CONFIG)
        .expect("temp config should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_sqlcomp"))
        .args(["compile", "--clean"])
        .current_dir(&config_dir)
        .output()
        .expect("sqlcomp compile should run");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("command `compile` is not implemented yet"),
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
