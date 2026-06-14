//! Inner domain types and language-neutral IR for `sqlcomp`.
//!
//! This crate is the innermost Clean Architecture boundary. It must not depend on
//! any other `sqlcomp-*` crate.

use std::path::{Path, PathBuf};

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

/// Dummy resolved compilation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationPlan;

/// Dummy query identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryId;

/// Dummy raw query extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawQuery;

/// Dummy dialect analysis result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzedQuery;

/// Dummy database metadata description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbQueryMetadata;

/// Dummy language-neutral compiled query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledQuery;

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
