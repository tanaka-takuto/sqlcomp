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
    fn parses_typescript_type_mapping_config() {
        let config = JsoncConfigLoader::parse_str(
            r#"
{
  "source": { "include": ["sql/**/*.sql"] },
  "output": { "dir": "src/generated/sqlay" },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "core": {
          "decimal": "number",
          "int64": "number"
        },
        "columns": {
          "orders.total_amount": {
            "type": "MoneyAmount",
            "import": {
              "from": "@/domain/money",
              "name": "MoneyAmount"
            }
          },
          "billing.orders.status": "BillingOrderStatus"
        },
        "builders": {
          "listOrders": {
            "fields": {
              "totalAmount": "MoneyAmount"
            },
            "params": {
              "minimumAmount": {
                "type": "MoneyAmount",
                "import": {
                  "from": "@/domain/money",
                  "name": "MoneyAmount"
                }
              }
            },
            "repeats": {
              "lineItems": {
                "fields": {
                  "unitPrice": "MoneyAmount"
                }
              }
            }
          }
        }
      }
    }
  }
}
"#,
        )
        .expect("TypeScript type mapping config should parse");
        let mapping = config.target().typescript().type_mapping();

        assert_core_type_mapping(mapping);
        assert_column_type_mapping(mapping);
        assert_builder_type_mapping(mapping);
    }

    fn assert_core_type_mapping(mapping: &core::TypeScriptTypeMappingConfig) {
        assert_eq!(mapping.core().len(), 2);
        let decimal = mapping
            .core()
            .iter()
            .find(|entry| entry.core_type() == core::CoreType::Decimal)
            .expect("decimal override should be parsed");
        assert_eq!(decimal.type_override().type_name(), "number");
        let int64 = mapping
            .core()
            .iter()
            .find(|entry| entry.core_type() == core::CoreType::Int64)
            .expect("int64 override should be parsed");
        assert_eq!(int64.type_override().type_name(), "number");
    }

    fn assert_column_type_mapping(mapping: &core::TypeScriptTypeMappingConfig) {
        assert_eq!(mapping.columns().len(), 2);
        let total_amount = mapping
            .columns()
            .iter()
            .find(|column| column.reference().column() == "total_amount")
            .expect("table.column override should be parsed");
        assert_eq!(total_amount.reference().database(), None);
        assert_eq!(total_amount.reference().table(), "orders");
        assert_eq!(total_amount.reference().column(), "total_amount");
        let column_import = total_amount
            .type_override()
            .import()
            .expect("column override should carry import metadata");
        assert_eq!(column_import.from(), "@/domain/money");
        assert_eq!(column_import.name(), "MoneyAmount");
        let status = mapping
            .columns()
            .iter()
            .find(|column| column.reference().column() == "status")
            .expect("database.table.column override should be parsed");
        assert_eq!(status.reference().database(), Some("billing"));
        assert_eq!(status.reference().table(), "orders");
        assert_eq!(status.reference().column(), "status");
    }

    fn assert_builder_type_mapping(mapping: &core::TypeScriptTypeMappingConfig) {
        assert_eq!(mapping.builders().len(), 1);
        let builder = mapping
            .builders()
            .iter()
            .find(|entry| entry.builder_id() == "listOrders")
            .expect("builder override should be parsed");
        let total_amount_field = builder
            .fields()
            .iter()
            .find(|field| field.name() == "totalAmount")
            .expect("builder field override should be parsed");
        assert_eq!(
            total_amount_field.type_override().type_name(),
            "MoneyAmount"
        );
        let minimum_amount = builder
            .params()
            .iter()
            .find(|param| param.name() == "minimumAmount")
            .expect("builder param override should be parsed");
        assert_eq!(minimum_amount.type_override().type_name(), "MoneyAmount");
        let minimum_amount_import = minimum_amount
            .type_override()
            .import()
            .expect("builder param override should carry import metadata");
        assert_eq!(minimum_amount_import.from(), "@/domain/money");
        assert_eq!(minimum_amount_import.name(), "MoneyAmount");
        let line_items = builder
            .repeats()
            .iter()
            .find(|repeat| repeat.repeat_id() == "lineItems")
            .expect("repeat override should be parsed");
        let unit_price = line_items
            .fields()
            .iter()
            .find(|field| field.name() == "unitPrice")
            .expect("repeat field override should be parsed");
        assert_eq!(unit_price.type_override().type_name(), "MoneyAmount");
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

    #[test]
    fn skips_typescript_validation_when_target_language_is_invalid() {
        let report = JsoncConfigLoader::parse_str(
            r#"
{
  "source": { "include": ["sql/**/*.sql"] },
  "output": { "dir": "src/generated/sqlay" },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "go",
    "typescript": {
      "typeMapping": {
        "core": {
          "money": "MoneyAmount"
        }
      }
    }
  }
}
"#,
        )
        .expect_err("unsupported target language should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains(
            "unsupported config field `target.language` value `go`; supported value is `typescript`"
        ));
        assert!(!messages.contains("target.typescript"));
    }

    #[test]
    fn rejects_invalid_typescript_type_mapping_config() {
        let report = JsoncConfigLoader::parse_str(
            r#"
{
  "source": { "include": ["sql/**/*.sql"] },
  "output": { "dir": "src/generated/sqlay" },
  "database": {
    "dialect": "mysql",
    "urlEnv": "DATABASE_URL"
  },
  "target": {
    "language": "typescript",
    "typescript": {
      "typeMapping": {
        "core": {
          "money": "MoneyAmount"
        },
        "columns": {
          "orders": "OrderType"
        },
        "builders": {
          "listOrders": {
            "fields": {
              "totalAmount": {
                "type": "MoneyAmount | null"
              }
            },
            "params": {
              "minimumAmount": {
                "type": "MoneyAmount",
                "import": {
                  "from": "./money",
                  "name": "Amount",
                  "alias": "Money"
                },
                "nullable": true
              }
            },
            "repeats": {
              "lineItems": {
                "params": {
                  "unitPrice": "MoneyAmount"
                }
              }
            }
          }
        }
      }
    }
  }
}
"#,
        )
        .expect_err("invalid TypeScript type mapping config should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(messages.contains("unsupported config field `target.typescript.typeMapping.core.money`; supported core type keys are `bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, `json`, and `unknown`"));
        assert!(messages.contains(
            "config field `target.typescript.typeMapping.columns.orders` must use `table.column` or `database.table.column`"
        ));
        assert!(messages.contains("config field `target.typescript.typeMapping.builders.listOrders.fields.totalAmount.type` value `MoneyAmount | null` must be a supported TypeScript primitive or portable type identifier matching `^[A-Za-z_][A-Za-z0-9_]*$`"));
        assert!(messages.contains("config field `target.typescript.typeMapping.builders.listOrders.params.minimumAmount.import.from` value `./money` must be a non-relative module specifier"));
        assert!(messages.contains("config field `target.typescript.typeMapping.builders.listOrders.params.minimumAmount.import.name` value `Amount` must match `type` value `MoneyAmount`; import aliases are not supported"));
        assert!(messages.contains("unknown config field `target.typescript.typeMapping.builders.listOrders.params.minimumAmount.import.alias`; supported fields are `from` and `name`"));
        assert!(messages.contains("unknown config field `target.typescript.typeMapping.builders.listOrders.params.minimumAmount.nullable`; supported fields are `type` and `import`"));
        assert!(messages.contains("unknown config field `target.typescript.typeMapping.builders.listOrders.repeats.lineItems.params`; supported fields are `fields`"));
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
