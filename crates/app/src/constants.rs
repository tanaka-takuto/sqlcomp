/// Standard project configuration file name.
pub const CONFIG_FILE_NAME: &str = "sqlcomp.config.json";

/// Starter configuration written by `sqlcomp init`.
pub const STARTER_CONFIG_TEMPLATE: &str = r#"{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": []
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
