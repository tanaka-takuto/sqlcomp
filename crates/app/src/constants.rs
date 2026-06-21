/// Standard project configuration file name.
pub const CONFIG_FILE_NAME: &str = "sqlay.config.json";

/// Starter configuration written by `sqlay init`.
pub const STARTER_CONFIG_TEMPLATE: &str = r#"{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": []
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
