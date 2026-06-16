//! JSONC configuration adapter.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sqlcomp_app::{CONFIG_FILE_NAME, ConfigLoader, ConfigTemplateWriter};
use sqlcomp_core as core;

/// JSONC-backed config loader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsoncConfigLoader {
    source: ConfigSource,
}

/// Filesystem-backed starter config writer.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsoncConfigTemplateWriter;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConfigSource {
    ExplicitPath(PathBuf),
    DiscoverFromCurrentDir,
    DiscoverFrom(PathBuf),
}

impl JsoncConfigLoader {
    /// Build a loader for an explicit config file path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ConfigSource::ExplicitPath(path.into()),
        }
    }

    /// Build a loader that discovers `sqlcomp.config.json` from the process
    /// current directory upward.
    #[must_use]
    pub const fn discover_from_current_dir() -> Self {
        Self {
            source: ConfigSource::DiscoverFromCurrentDir,
        }
    }

    /// Build a loader that discovers `sqlcomp.config.json` from a directory
    /// upward.
    #[must_use]
    pub fn discover_from(start_dir: impl Into<PathBuf>) -> Self {
        Self {
            source: ConfigSource::DiscoverFrom(start_dir.into()),
        }
    }

    /// Return the explicit path this loader reads when one was configured.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match &self.source {
            ConfigSource::ExplicitPath(path) => Some(path),
            ConfigSource::DiscoverFromCurrentDir | ConfigSource::DiscoverFrom(_) => None,
        }
    }

    /// Parse and validate JSONC configuration content.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the content cannot be parsed as JSONC or when
    /// required fields are missing or unsupported.
    pub fn parse_str(source: &str) -> core::DiagnosticResult<core::ProjectConfig> {
        parse_config(source, None, Path::new(".").to_path_buf())
    }

    /// Parse and validate JSONC configuration content with an explicit config
    /// directory.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the content cannot be parsed as JSONC or when
    /// required fields are missing or unsupported.
    pub fn parse_str_from_dir(
        source: &str,
        config_dir: impl Into<PathBuf>,
    ) -> core::DiagnosticResult<core::ProjectConfig> {
        parse_config(source, None, config_dir.into())
    }
}

impl Default for JsoncConfigLoader {
    fn default() -> Self {
        Self::discover_from_current_dir()
    }
}

impl ConfigLoader for JsoncConfigLoader {
    fn load(&self) -> core::DiagnosticResult<core::ProjectConfig> {
        let path = self.resolve_path()?;
        let source = fs::read_to_string(&path).map_err(|error| {
            single_error_report(
                format!("failed to read config file `{}`: {error}", path.display()),
                Some(core::SourceLocation::for_path(path.clone())),
            )
        })?;

        parse_config(&source, Some(&path), config_dir_from_path(&path))
    }
}

impl ConfigTemplateWriter for JsoncConfigTemplateWriter {
    fn write_new(&self, path: &Path, contents: &str) -> core::DiagnosticResult<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    single_error_report(
                        format!(
                            "refusing to overwrite existing config file `{}`",
                            path.display()
                        ),
                        Some(core::SourceLocation::for_path(path)),
                    )
                } else {
                    single_error_report(
                        format!("failed to create config file `{}`: {error}", path.display()),
                        Some(core::SourceLocation::for_path(path)),
                    )
                }
            })?;

        file.write_all(contents.as_bytes()).map_err(|error| {
            single_error_report(
                format!("failed to write config file `{}`: {error}", path.display()),
                Some(core::SourceLocation::for_path(path)),
            )
        })
    }
}

impl JsoncConfigLoader {
    fn resolve_path(&self) -> core::DiagnosticResult<PathBuf> {
        match &self.source {
            ConfigSource::ExplicitPath(path) => Ok(path.clone()),
            ConfigSource::DiscoverFromCurrentDir => {
                let start_dir = std::env::current_dir().map_err(|error| {
                    single_error_report(
                        format!(
                            "failed to determine current directory while searching for `{CONFIG_FILE_NAME}`: {error}"
                        ),
                        None,
                    )
                })?;

                discover_config_path(&start_dir)
            }
            ConfigSource::DiscoverFrom(start_dir) => discover_config_path(start_dir),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProjectConfig {
    source: Option<RawSourceConfig>,
    output: Option<RawOutputConfig>,
    database: Option<RawDatabaseConfig>,
    target: Option<RawTargetConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSourceConfig {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutputConfig {
    dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawDatabaseConfig {
    dialect: Option<String>,
    url_env: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTargetConfig {
    language: Option<String>,
}

fn parse_config(
    source: &str,
    path: Option<&Path>,
    config_dir: PathBuf,
) -> core::DiagnosticResult<core::ProjectConfig> {
    let normalized = normalize_jsonc(source).map_err(|message| {
        single_error_report(
            format!("failed to parse `sqlcomp.config.json` as JSONC: {message}"),
            path.map(core::SourceLocation::for_path),
        )
    })?;

    let raw = serde_json::from_str::<RawProjectConfig>(&normalized).map_err(|error| {
        let location = parse_error_location(path, &error);
        single_error_report(
            format!("failed to parse `sqlcomp.config.json` as JSONC: {error}"),
            location,
        )
    })?;

    validate_config(raw, path, config_dir)
}

fn validate_config(
    raw: RawProjectConfig,
    path: Option<&Path>,
    config_dir: PathBuf,
) -> core::DiagnosticResult<core::ProjectConfig> {
    let location = path.map(core::SourceLocation::for_path);
    let mut diagnostics = core::DiagnosticReport::default();

    let source = validate_source(raw.source, location.as_ref(), &mut diagnostics);
    let output = validate_output(raw.output, location.as_ref(), &mut diagnostics);
    let database = validate_database(raw.database, location.as_ref(), &mut diagnostics);
    let target = validate_target(raw.target, location.as_ref(), &mut diagnostics);

    if diagnostics.is_empty() {
        if let (Some(source), Some(output), Some(database), Some(target)) =
            (source, output, database, target)
        {
            Ok(core::ProjectConfig::new(
                config_dir, source, output, database, target,
            ))
        } else {
            Err(diagnostics)
        }
    } else {
        Err(diagnostics)
    }
}

fn config_dir_from_path(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn discover_config_path(start_dir: &Path) -> core::DiagnosticResult<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let candidate = current.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Ok(candidate);
        }

        if !current.pop() {
            break;
        }
    }

    Err(single_error_report(
        format!(
            "failed to find `{CONFIG_FILE_NAME}` from `{}` or any parent directory",
            start_dir.display()
        ),
        None,
    ))
}

fn validate_source(
    raw: Option<RawSourceConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::SourceConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "source.include", location);
        return None;
    };

    let include = required_field(raw.include, "source.include", location, diagnostics)?;
    let exclude = raw.exclude.unwrap_or_default();

    Some(core::SourceConfig::new(include, exclude))
}

fn validate_output(
    raw: Option<RawOutputConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::OutputConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "output.dir", location);
        return None;
    };

    let dir = required_field(raw.dir, "output.dir", location, diagnostics)?;

    Some(core::OutputConfig::new(dir))
}

fn validate_database(
    raw: Option<RawDatabaseConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::DatabaseConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "database.dialect", location);
        push_missing_field(diagnostics, "database.urlEnv", location);
        return None;
    };

    let dialect = required_field(raw.dialect, "database.dialect", location, diagnostics)
        .and_then(|value| validate_database_dialect(&value, location, diagnostics));
    let url_env = required_field(raw.url_env, "database.urlEnv", location, diagnostics);

    Some(core::DatabaseConfig::new(dialect?, url_env?))
}

fn validate_target(
    raw: Option<RawTargetConfig>,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TargetConfig> {
    let Some(raw) = raw else {
        push_missing_field(diagnostics, "target.language", location);
        return None;
    };

    let language = required_field(raw.language, "target.language", location, diagnostics)
        .and_then(|value| validate_target_language(&value, location, diagnostics));

    Some(core::TargetConfig::new(language?))
}

fn validate_database_dialect(
    value: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::DatabaseDialect> {
    if value == "mysql" {
        Some(core::DatabaseDialect::MySql)
    } else {
        push_error(
            diagnostics,
            format!(
                "unsupported config field `database.dialect` value `{value}`; supported value is `mysql`"
            ),
            location,
        );
        None
    }
}

fn validate_target_language(
    value: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<core::TargetLanguage> {
    if value == "typescript" {
        Some(core::TargetLanguage::TypeScript)
    } else {
        push_error(
            diagnostics,
            format!(
                "unsupported config field `target.language` value `{value}`; supported value is `typescript`"
            ),
            location,
        );
        None
    }
}

fn required_field<T>(
    value: Option<T>,
    name: &str,
    location: Option<&core::SourceLocation>,
    diagnostics: &mut core::DiagnosticReport,
) -> Option<T> {
    if value.is_none() {
        push_missing_field(diagnostics, name, location);
    }

    value
}

fn push_missing_field(
    diagnostics: &mut core::DiagnosticReport,
    name: &str,
    location: Option<&core::SourceLocation>,
) {
    push_error(
        diagnostics,
        format!("missing required config field `{name}`"),
        location,
    );
}

fn push_error(
    diagnostics: &mut core::DiagnosticReport,
    message: impl Into<String>,
    location: Option<&core::SourceLocation>,
) {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = location {
        diagnostic = diagnostic.with_location(location.clone());
    }
    diagnostics.push(diagnostic);
}

fn single_error_report(
    message: impl Into<String>,
    location: Option<core::SourceLocation>,
) -> core::DiagnosticReport {
    let diagnostic = if let Some(location) = location {
        core::Diagnostic::error(message).with_location(location)
    } else {
        core::Diagnostic::error(message)
    };

    core::DiagnosticReport::new(diagnostic)
}

fn parse_error_location(
    path: Option<&Path>,
    error: &serde_json::Error,
) -> Option<core::SourceLocation> {
    let position = core::SourcePosition::one_based(error.line(), error.column())?;

    Some(path.map_or_else(
        || core::SourceLocation::from_range(core::SourceRange::point(position)),
        |path| core::SourceLocation::at_position(path, position),
    ))
}

fn normalize_jsonc(source: &str) -> Result<String, &'static str> {
    let without_comments = strip_jsonc_comments(source)?;
    Ok(remove_trailing_commas(&without_comments))
}

fn strip_jsonc_comments(source: &str) -> Result<String, &'static str> {
    let mut stripped = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(char) = chars.next() {
        if in_string {
            stripped.push(char);
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                in_string = false;
            }
            continue;
        }

        if char == '"' {
            in_string = true;
            stripped.push(char);
            continue;
        }

        if char == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    stripped.push(' ');
                    stripped.push(' ');
                    strip_line_comment(&mut chars, &mut stripped);
                }
                Some('*') => {
                    chars.next();
                    stripped.push(' ');
                    stripped.push(' ');
                    strip_block_comment(&mut chars, &mut stripped)?;
                }
                _ => stripped.push(char),
            }
        } else {
            stripped.push(char);
        }
    }

    Ok(stripped)
}

fn strip_line_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, stripped: &mut String) {
    for char in chars.by_ref() {
        if char == '\n' {
            stripped.push('\n');
            break;
        }

        stripped.push(if char == '\r' { '\r' } else { ' ' });
    }
}

fn strip_block_comment(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    stripped: &mut String,
) -> Result<(), &'static str> {
    while let Some(char) = chars.next() {
        if char == '*' && chars.peek().copied() == Some('/') {
            chars.next();
            stripped.push(' ');
            stripped.push(' ');
            return Ok(());
        }

        stripped.push(if matches!(char, '\n' | '\r') {
            char
        } else {
            ' '
        });
    }

    Err("unterminated block comment")
}

fn remove_trailing_commas(source: &str) -> String {
    let chars = source.chars().collect::<Vec<_>>();
    let mut normalized = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let char = chars[index];

        if in_string {
            normalized.push(char);
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if char == '"' {
            in_string = true;
            normalized.push(char);
            index += 1;
            continue;
        }

        if char == ','
            && next_significant_char(&chars, index + 1)
                .is_some_and(|next| matches!(next, '}' | ']'))
        {
            normalized.push(' ');
        } else {
            normalized.push(char);
        }

        index += 1;
    }

    normalized
}

fn next_significant_char(chars: &[char], start: usize) -> Option<char> {
    chars
        .iter()
        .skip(start)
        .copied()
        .find(|char| !char.is_whitespace())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{CONFIG_FILE_NAME, JsoncConfigLoader};
    use sqlcomp_app::{CompilationPlanner, ConfigLoader};
    use sqlcomp_core as core;

    const VALID_CONFIG: &str = r#"
{
  "source": {
    "include": ["sql/**/*.sql"],
    "exclude": ["sql/private/**/*.sql"]
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
    fn parses_valid_config() {
        let config = JsoncConfigLoader::parse_str(VALID_CONFIG).expect("valid config should parse");

        assert_eq!(config.config_dir(), Path::new("."));
        assert_eq!(config.source().include(), ["sql/**/*.sql"]);
        assert_eq!(config.source().exclude(), ["sql/private/**/*.sql"]);
        assert_eq!(config.output().dir(), "src/generated/sqlcomp");
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
    "dir": "src/generated/sqlcomp", /* trailing commas are allowed */
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
  "output": { "dir": "src/generated/sqlcomp" },
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
    fn default_uses_current_directory_discovery() {
        assert_eq!(
            JsoncConfigLoader::default(),
            JsoncConfigLoader::discover_from_current_dir()
        );
    }

    #[test]
    fn discovers_config_from_config_directory() {
        let config_dir = unique_temp_dir("sqlcomp-config-discovery-root");
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
        let config_dir = unique_temp_dir("sqlcomp-config-discovery-nested");
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
    fn nested_discovery_plans_paths_from_config_directory() {
        let config_dir = unique_temp_dir("sqlcomp-config-plan-nested");
        let child_dir = config_dir.join("packages").join("api").join("src");
        fs::create_dir_all(&child_dir).expect("temp child dir should be created");
        fs::write(config_dir.join(CONFIG_FILE_NAME), VALID_CONFIG)
            .expect("temp config should be written");

        let planner = sqlcomp_app::DefaultCompilationPlanner;
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
            config_dir.join("src/generated/sqlcomp")
        );
        assert_eq!(
            nested_plan.source_relative_path(config_dir.join("sql/nested/users/list.sql")),
            Some(PathBuf::from("sql/nested/users/list.sql"))
        );

        fs::remove_dir_all(config_dir).expect("temp config dir should be removed");
    }

    #[test]
    fn reports_when_discovery_does_not_find_config() {
        let start_dir = unique_temp_dir("sqlcomp-config-discovery-missing")
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
        let config_dir = unique_temp_dir("sqlcomp-config-explicit-bypass");
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

    #[test]
    fn rejects_unknown_fields() {
        let report = JsoncConfigLoader::parse_str(
            r#"
{
  "source": {
    "include": ["sql/**/*.sql"],
    "excludes": ["sql/private/**/*.sql"]
  },
  "output": { "dir": "src/generated/sqlcomp" },
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

    fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
        report
            .diagnostics()
            .iter()
            .map(core::Diagnostic::message)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn unique_temp_config_path() -> PathBuf {
        unique_temp_dir("sqlcomp-config-jsonc")
            .join("packages")
            .join("api")
            .join(CONFIG_FILE_NAME)
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
    }
}
