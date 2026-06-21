use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use sqlay_app::{CONFIG_FILE_NAME, CompilationPlanner, ConfigLoader};
use sqlay_core as core;

use super::JsoncConfigLoader;

const VALID_CONFIG: &str = r#"
{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": ["sql/private/**/*.sql"]
  },
  "output": {
    "dir": "src/generated/sqlay"
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

mod parser {
    use super::*;

    #[test]
    fn parses_valid_config() {
        let config = JsoncConfigLoader::parse_str(VALID_CONFIG).expect("valid config should parse");

        assert_eq!(config.config_dir(), Path::new("."));
        assert_eq!(config.source().include(), ["sql/**/*.sql"]);
        assert_eq!(config.source().exclude(), ["sql/private/**/*.sql"]);
        assert_eq!(config.output().dir(), "src/generated/sqlay");
        assert_eq!(config.database().dialect(), core::DatabaseDialect::MySql);
        assert_eq!(config.database().url_env(), "DATABASE_URL");
        assert_eq!(config.target().language(), core::TargetLanguage::TypeScript);
    }

    #[test]
    fn accepts_comments_and_trailing_commas() {
        let config = JsoncConfigLoader::parse_str(
            r#"
{
  // Source globs are config-file-relative.
  "source": {
    "include": ["sql/**/*.sql",],
  },
  "output": {
    "dir": "src/generated/sqlay", /* trailing commas are allowed */
  },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL",
  },
  "target": {
    "language": "typescript",
  },
}
"#,
        )
        .expect("JSONC config should parse");

        assert_eq!(config.source().include(), ["sql/**/*.sql"]);
        assert!(config.source().exclude().is_empty());
    }

    #[test]
    fn rejects_unknown_fields() {
        let report = JsoncConfigLoader::parse_str(
            r#"
{
  "source": {
    "include": ["sql/**/*.sql"],
    "excludes": ["sql/private/**/*.sql"]
  },
  "output": { "dir": "src/generated/sqlay" },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": { "language": "typescript" }
}
"#,
        )
        .expect_err("unknown fields should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains("unknown field `excludes`"));
    }
}

mod validation {
    use super::*;

    #[test]
    fn rejects_missing_required_fields() {
        let report = JsoncConfigLoader::parse_str(
            r#"
{
  "source": {},
  "output": {},
  "database": {},
  "target": {}
}
"#,
        )
        .expect_err("missing fields should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains("missing required config field `source.include`"));
        assert!(messages.contains("missing required config field `output.dir`"));
        assert!(messages.contains("missing required config field `database.dialect`"));
        assert!(messages.contains("missing required config field `database.urlEnv`"));
        assert!(messages.contains("missing required config field `target.language`"));
    }

    #[test]
    fn rejects_missing_top_level_sections_as_required_fields() {
        let report = JsoncConfigLoader::parse_str("{}")
            .expect_err("missing top-level sections should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains("missing required config field `source.include`"));
        assert!(messages.contains("missing required config field `output.dir`"));
        assert!(messages.contains("missing required config field `database.dialect`"));
        assert!(messages.contains("missing required config field `database.urlEnv`"));
        assert!(messages.contains("missing required config field `target.language`"));
    }

    #[test]
    fn rejects_unsupported_dialect_and_target() {
        let report = JsoncConfigLoader::parse_str(
            r#"
{
  "source": { "include": ["sql/**/*.sql"] },
  "output": { "dir": "src/generated/sqlay" },
  "database": {
    "dialect": "postgres",
    "urlEnv": "DATABASE_URL"
  },
  "target": { "language": "go" }
}
"#,
        )
        .expect_err("unsupported values should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(
            messages.contains("unsupported config field `database.dialect` value `postgres`; supported value is `mysql`")
        );
        assert!(messages.contains(
            "unsupported config field `target.language` value `go`; supported value is `typescript`"
        ));
    }
}

mod discovery {
    use super::*;

    #[test]
    fn default_uses_current_directory_discovery() {
        assert_eq!(
            JsoncConfigLoader::default(),
            JsoncConfigLoader::discover_from_current_dir()
        );
    }

    #[test]
    fn discovers_config_from_config_directory() {
        let config_dir = unique_temp_dir("sqlay-config-discovery-root");
        fs::create_dir_all(&config_dir).expect("temp config dir should be created");
        fs::write(config_dir.join(CONFIG_FILE_NAME), VALID_CONFIG)
            .expect("temp config should be written");

        let config = JsoncConfigLoader::discover_from(&config_dir)
            .load()
            .expect("valid discovered config should load");

        assert_eq!(config.config_dir(), config_dir);

        fs::remove_dir_all(config_dir).expect("temp config dir should be removed");
    }

    #[test]
    fn discovers_config_from_nested_child_directory() {
        let config_dir = unique_temp_dir("sqlay-config-discovery-nested");
        let child_dir = config_dir.join("packages").join("api").join("sql");
        fs::create_dir_all(&child_dir).expect("temp child dir should be created");
        fs::write(config_dir.join(CONFIG_FILE_NAME), VALID_CONFIG)
            .expect("temp config should be written");

        let config = JsoncConfigLoader::discover_from(child_dir)
            .load()
            .expect("valid discovered config should load");

        assert_eq!(config.config_dir(), config_dir);

        fs::remove_dir_all(config_dir).expect("temp config dir should be removed");
    }

    #[test]
    fn reports_when_discovery_does_not_find_config() {
        let start_dir = unique_temp_dir("sqlay-config-discovery-missing")
            .join("packages")
            .join("api");
        fs::create_dir_all(&start_dir).expect("temp child dir should be created");

        let report = JsoncConfigLoader::discover_from(&start_dir)
            .load()
            .expect_err("missing discovered config should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains(&format!(
            "failed to find `{CONFIG_FILE_NAME}` from `{}` or any parent directory",
            start_dir.display()
        )));

        fs::remove_dir_all(
            start_dir
                .parent()
                .and_then(Path::parent)
                .expect("temp root should exist"),
        )
        .expect("temp config tree should be removed");
    }

    #[test]
    fn explicit_path_bypasses_upward_discovery() {
        let config_dir = unique_temp_dir("sqlay-config-explicit-bypass");
        let child_dir = config_dir.join("packages").join("api");
        let explicit_path = child_dir.join(CONFIG_FILE_NAME);
        fs::create_dir_all(&child_dir).expect("temp child dir should be created");
        fs::write(config_dir.join(CONFIG_FILE_NAME), VALID_CONFIG)
            .expect("parent config should be written");

        let report = JsoncConfigLoader::new(&explicit_path)
            .load()
            .expect_err("explicit missing config should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains(&format!(
            "failed to read config file `{}`",
            explicit_path.display()
        )));

        fs::remove_dir_all(config_dir).expect("temp config dir should be removed");
    }
}

mod paths {
    use super::*;

    #[test]
    fn load_retains_config_file_directory() {
        let config_path = unique_temp_config_path();
        let config_dir = config_path
            .parent()
            .expect("temp config path should have a parent")
            .to_path_buf();
        fs::create_dir_all(&config_dir).expect("temp config dir should be created");
        fs::write(&config_path, VALID_CONFIG).expect("temp config should be written");

        let config = JsoncConfigLoader::new(&config_path)
            .load()
            .expect("valid config file should load");

        assert_eq!(config.config_dir(), config_dir);

        fs::remove_file(&config_path).expect("temp config should be removed");
        fs::remove_dir_all(
            config_dir
                .parent()
                .expect("temp package dir should have a parent"),
        )
        .expect("temp config tree should be removed");
    }

    #[test]
    fn nested_discovery_plans_paths_from_config_directory() {
        let config_dir = unique_temp_dir("sqlay-config-plan-nested");
        let child_dir = config_dir.join("packages").join("api").join("src");
        fs::create_dir_all(&child_dir).expect("temp child dir should be created");
        fs::write(config_dir.join(CONFIG_FILE_NAME), VALID_CONFIG)
            .expect("temp config should be written");

        let planner = sqlay_app::DefaultCompilationPlanner;
        let root_config = JsoncConfigLoader::discover_from(&config_dir)
            .load()
            .expect("config should load from root");
        let nested_config = JsoncConfigLoader::discover_from(&child_dir)
            .load()
            .expect("config should load from nested child");
        let root_plan = planner
            .plan(&root_config)
            .expect("root config should produce a plan");
        let nested_plan = planner
            .plan(&nested_config)
            .expect("nested config should produce a plan");

        assert_eq!(root_plan, nested_plan);
        assert_eq!(
            nested_plan.source_include(),
            [config_dir.join("sql/**/*.sql")]
        );
        assert_eq!(
            nested_plan.source_exclude(),
            [config_dir.join("sql/private/**/*.sql")]
        );
        assert_eq!(
            nested_plan.output_dir(),
            config_dir.join("src/generated/sqlay")
        );
        assert_eq!(
            nested_plan.source_relative_path(config_dir.join("sql/nested/users/list.sql")),
            Some(PathBuf::from("sql/nested/users/list.sql"))
        );

        fs::remove_dir_all(config_dir).expect("temp config dir should be removed");
    }
}

fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
    report
        .diagnostics()
        .iter()
        .map(core::Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n")
}

fn unique_temp_config_path() -> PathBuf {
    unique_temp_dir("sqlay-config-jsonc")
        .join("packages")
        .join("api")
        .join(CONFIG_FILE_NAME)
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    static NEXT_TEMP_DIR_ID: AtomicUsize = AtomicUsize::new(0);

    let counter = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "{prefix}-{}-{unique}-{counter}",
        std::process::id()
    ))
}
