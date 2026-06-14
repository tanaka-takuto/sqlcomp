//! Inner domain types and language-neutral IR for `sqlcomp`.
//!
//! This crate is the innermost Clean Architecture boundary. It must not depend on
//! any other `sqlcomp-*` crate.

use std::path::{Component, Path, PathBuf};

mod reporting;

pub use reporting::{
    Diagnostic, DiagnosticReport, DiagnosticResult, DiagnosticSeverity, SourceLocation,
    SourcePosition, SourceRange,
};

/// Validated project configuration accepted by application use cases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectConfig {
    config_dir: PathBuf,
    source: SourceConfig,
    output: OutputConfig,
    database: DatabaseConfig,
    target: TargetConfig,
}

impl ProjectConfig {
    /// Build a validated project configuration from its sections.
    #[must_use]
    pub const fn new(
        config_dir: PathBuf,
        source: SourceConfig,
        output: OutputConfig,
        database: DatabaseConfig,
        target: TargetConfig,
    ) -> Self {
        Self {
            config_dir,
            source,
            output,
            database,
            target,
        }
    }

    /// Directory containing `sqlcomp.config.json`.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Source file selection settings.
    #[must_use]
    pub const fn source(&self) -> &SourceConfig {
        &self.source
    }

    /// Generated output settings.
    #[must_use]
    pub const fn output(&self) -> &OutputConfig {
        &self.output
    }

    /// Database metadata settings.
    #[must_use]
    pub const fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    /// Target-language settings.
    #[must_use]
    pub const fn target(&self) -> &TargetConfig {
        &self.target
    }
}

/// Source file selection settings from `sqlcomp.config.json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceConfig {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl SourceConfig {
    /// Build source file selection settings.
    #[must_use]
    pub const fn new(include: Vec<String>, exclude: Vec<String>) -> Self {
        Self { include, exclude }
    }

    /// Include glob patterns relative to the configuration file directory.
    #[must_use]
    pub fn include(&self) -> &[String] {
        &self.include
    }

    /// Exclude glob patterns relative to the configuration file directory.
    #[must_use]
    pub fn exclude(&self) -> &[String] {
        &self.exclude
    }
}

/// Generated output settings from `sqlcomp.config.json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputConfig {
    dir: String,
}

impl OutputConfig {
    /// Build generated output settings.
    #[must_use]
    pub const fn new(dir: String) -> Self {
        Self { dir }
    }

    /// Output directory relative to the configuration file directory.
    #[must_use]
    pub fn dir(&self) -> &str {
        &self.dir
    }
}

/// Database metadata settings from `sqlcomp.config.json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseConfig {
    dialect: DatabaseDialect,
    url_env: String,
}

impl DatabaseConfig {
    /// Build database metadata settings.
    #[must_use]
    pub const fn new(dialect: DatabaseDialect, url_env: String) -> Self {
        Self { dialect, url_env }
    }

    /// Configured database dialect.
    #[must_use]
    pub const fn dialect(&self) -> DatabaseDialect {
        self.dialect
    }

    /// Environment variable name that contains the database URL.
    #[must_use]
    pub fn url_env(&self) -> &str {
        &self.url_env
    }
}

/// Supported MVP database dialects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatabaseDialect {
    /// Official `MySQL` 8.x.
    MySql,
}

impl DatabaseDialect {
    /// Return the stable configuration spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MySql => "mysql",
        }
    }
}

/// Target-language settings from `sqlcomp.config.json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetConfig {
    language: TargetLanguage,
}

impl TargetConfig {
    /// Build target-language settings.
    #[must_use]
    pub const fn new(language: TargetLanguage) -> Self {
        Self { language }
    }

    /// Configured target language.
    #[must_use]
    pub const fn language(&self) -> TargetLanguage {
        self.language
    }
}

/// Supported MVP target languages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetLanguage {
    /// `TypeScript` SQL builder generation.
    TypeScript,
}

impl TargetLanguage {
    /// Return the stable configuration spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
        }
    }
}

/// Resolved compilation work order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationPlan {
    config_dir: PathBuf,
    source_include: Vec<PathBuf>,
    source_exclude: Vec<PathBuf>,
    output_dir: PathBuf,
    database: DatabaseConfig,
    target: TargetConfig,
}

impl CompilationPlan {
    /// Build a resolved compilation plan.
    #[must_use]
    pub const fn new(
        config_dir: PathBuf,
        source_include: Vec<PathBuf>,
        source_exclude: Vec<PathBuf>,
        output_dir: PathBuf,
        database: DatabaseConfig,
        target: TargetConfig,
    ) -> Self {
        Self {
            config_dir,
            source_include,
            source_exclude,
            output_dir,
            database,
            target,
        }
    }

    /// Directory containing `sqlcomp.config.json`.
    #[must_use]
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Include glob patterns resolved relative to the configuration directory.
    #[must_use]
    pub fn source_include(&self) -> &[PathBuf] {
        &self.source_include
    }

    /// Exclude glob patterns resolved relative to the configuration directory.
    #[must_use]
    pub fn source_exclude(&self) -> &[PathBuf] {
        &self.source_exclude
    }

    /// Generated output directory resolved relative to the configuration directory.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Database metadata settings for this compile run.
    #[must_use]
    pub const fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    /// Target-language settings for this compile run.
    #[must_use]
    pub const fn target(&self) -> &TargetConfig {
        &self.target
    }

    /// Return a source path relative to the configuration directory.
    #[must_use]
    pub fn source_relative_path(&self, source_path: impl AsRef<Path>) -> Option<PathBuf> {
        let relative_path = source_path
            .as_ref()
            .strip_prefix(&self.config_dir)
            .ok()?
            .to_path_buf();

        is_safe_relative_path(&relative_path).then_some(relative_path)
    }
}

fn is_safe_relative_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::CurDir | Component::Normal(_)))
}

/// Dummy query identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryId;

/// Metadata parsed from an MVP `type: query` annotation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryMetadata {
    id: String,
    cardinality: Option<Cardinality>,
}

impl QueryMetadata {
    /// Build parsed query metadata.
    #[must_use]
    pub const fn new(id: String, cardinality: Option<Cardinality>) -> Self {
        Self { id, cardinality }
    }

    /// Query ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Optional cardinality override from source metadata.
    #[must_use]
    pub const fn cardinality(&self) -> Option<Cardinality> {
        self.cardinality
    }
}

/// Raw query extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawQuery {
    metadata: QueryMetadata,
    sql: String,
    source_location: Option<SourceLocation>,
}

impl RawQuery {
    /// Build a raw query from parsed metadata and SQL text.
    #[must_use]
    pub const fn new(metadata: QueryMetadata, sql: String) -> Self {
        Self {
            metadata,
            sql,
            source_location: None,
        }
    }

    /// Attach source location context for diagnostics.
    #[must_use]
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = Some(location);
        self
    }

    /// Query metadata parsed from the source annotation.
    #[must_use]
    pub const fn metadata(&self) -> &QueryMetadata {
        &self.metadata
    }

    /// Raw SQL body for this query.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Optional source location for the SQL body.
    #[must_use]
    pub const fn source_location(&self) -> Option<&SourceLocation> {
        self.source_location.as_ref()
    }
}

/// Dialect analysis result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzedQuery {
    cardinality: Cardinality,
}

impl AnalyzedQuery {
    /// Build dialect analysis facts for a query.
    #[must_use]
    pub const fn new(cardinality: Cardinality) -> Self {
        Self { cardinality }
    }

    /// Cardinality inferred by dialect analysis.
    #[must_use]
    pub const fn cardinality(&self) -> Cardinality {
        self.cardinality
    }
}

/// Dummy database metadata description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbQueryMetadata;

/// Language-neutral compiled query facts available before full row IR exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledQuery {
    cardinality: Cardinality,
}

impl CompiledQuery {
    /// Build a compiled query with resolved cardinality.
    #[must_use]
    pub const fn new(cardinality: Cardinality) -> Self {
        Self { cardinality }
    }

    /// Cardinality after explicit metadata overrides have been applied.
    #[must_use]
    pub const fn cardinality(&self) -> Cardinality {
        self.cardinality
    }
}

/// Dummy generated file set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFiles;

/// Query cardinality in generated output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cardinality {
    /// A query returns zero or one row.
    One,
    /// A query returns zero or more rows.
    Many,
}

/// Language-neutral type classification for generated output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreType {
    /// Boolean value.
    Bool,
    /// 32-bit integer value.
    Int32,
    /// 64-bit integer value.
    Int64,
    /// 64-bit floating-point value.
    Float64,
    /// Decimal value.
    Decimal,
    /// Text value.
    String,
    /// Binary value.
    Bytes,
    /// Date value.
    Date,
    /// Date-time value.
    DateTime,
    /// JSON value.
    Json,
    /// Unknown database type.
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        AnalyzedQuery, Cardinality, CompilationPlan, DatabaseConfig, DatabaseDialect,
        QueryMetadata, RawQuery, SourceLocation, SourcePosition, SourceRange, TargetConfig,
        TargetLanguage,
    };

    #[test]
    fn source_relative_path_returns_config_relative_path() {
        let config_dir = PathBuf::from("/tmp/sqlcomp-project");
        let plan = compilation_plan(config_dir.clone());

        let relative_path = plan
            .source_relative_path(config_dir.join("sql/users/list.sql"))
            .expect("source path should be inside config dir");

        assert_eq!(relative_path, Path::new("sql/users/list.sql"));
    }

    #[test]
    fn source_relative_path_rejects_parent_dir_after_config_prefix() {
        let config_dir = PathBuf::from("/tmp/sqlcomp-project");
        let plan = compilation_plan(config_dir.clone());

        assert_eq!(
            plan.source_relative_path(config_dir.join("../shared/users.sql")),
            None
        );
    }

    #[test]
    fn raw_query_preserves_metadata_sql_and_optional_source_location() {
        let location = SourceLocation::at_range(
            "sql/users.sql",
            SourceRange::point(
                SourcePosition::one_based(8, 1).expect("test position should be valid"),
            ),
        );
        let query = RawQuery::new(
            QueryMetadata::new("listUsers".to_owned(), Some(Cardinality::One)),
            "SELECT id FROM users;".to_owned(),
        )
        .with_source_location(location.clone());

        assert_eq!(query.metadata().id(), "listUsers");
        assert_eq!(query.metadata().cardinality(), Some(Cardinality::One));
        assert_eq!(query.sql(), "SELECT id FROM users;");
        assert_eq!(query.source_location(), Some(&location));
    }

    #[test]
    fn analyzed_query_exposes_inferred_cardinality() {
        let analysis = AnalyzedQuery::new(Cardinality::Many);

        assert_eq!(analysis.cardinality(), Cardinality::Many);
    }

    fn compilation_plan(config_dir: PathBuf) -> CompilationPlan {
        CompilationPlan::new(
            config_dir,
            vec![PathBuf::from("sql/**/*.sql")],
            Vec::new(),
            PathBuf::from("src/generated/sqlcomp"),
            DatabaseConfig::new(DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
            TargetConfig::new(TargetLanguage::TypeScript),
        )
    }
}
