use std::path::{Path, PathBuf};

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

    /// Directory containing `sqlay.config.json`.
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

/// Source file selection settings from `sqlay.config.json`.
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

/// Generated output settings from `sqlay.config.json`.
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

/// Database metadata settings from `sqlay.config.json`.
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

/// Supported database dialects.
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

/// Target-language settings from `sqlay.config.json`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetConfig {
    language: TargetLanguage,
    typescript: TypeScriptTargetConfig,
}

impl TargetConfig {
    /// Build target-language settings.
    #[must_use]
    pub const fn new(language: TargetLanguage) -> Self {
        Self {
            language,
            typescript: TypeScriptTargetConfig::empty(),
        }
    }

    /// Build target-language settings with TypeScript-specific settings.
    #[must_use]
    pub fn with_typescript_type_mapping(
        mut self,
        type_mapping: TypeScriptTypeMappingConfig,
    ) -> Self {
        self.typescript = TypeScriptTargetConfig::new(type_mapping);
        self
    }

    /// Configured target language.
    #[must_use]
    pub const fn language(&self) -> TargetLanguage {
        self.language
    }

    /// TypeScript target settings.
    #[must_use]
    pub const fn typescript(&self) -> &TypeScriptTargetConfig {
        &self.typescript
    }
}

/// Supported target languages.
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

/// TypeScript-specific target settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptTargetConfig {
    type_mapping: TypeScriptTypeMappingConfig,
}

impl TypeScriptTargetConfig {
    /// Build empty TypeScript target settings.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            type_mapping: TypeScriptTypeMappingConfig::empty(),
        }
    }

    /// Build TypeScript target settings.
    #[must_use]
    pub const fn new(type_mapping: TypeScriptTypeMappingConfig) -> Self {
        Self { type_mapping }
    }

    /// TypeScript type annotation override settings.
    #[must_use]
    pub const fn type_mapping(&self) -> &TypeScriptTypeMappingConfig {
        &self.type_mapping
    }
}

/// TypeScript type annotation override settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptTypeMappingConfig {
    core: Vec<CoreTypeOverride>,
    columns: Vec<ColumnTypeOverride>,
    builders: Vec<BuilderTypeOverrides>,
}

impl TypeScriptTypeMappingConfig {
    /// Build empty type mapping settings.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            core: Vec::new(),
            columns: Vec::new(),
            builders: Vec::new(),
        }
    }

    /// Build type mapping settings.
    #[must_use]
    pub const fn new(
        core: Vec<CoreTypeOverride>,
        columns: Vec<ColumnTypeOverride>,
        builders: Vec<BuilderTypeOverrides>,
    ) -> Self {
        Self {
            core,
            columns,
            builders,
        }
    }

    /// Broad Core type overrides.
    #[must_use]
    pub fn core(&self) -> &[CoreTypeOverride] {
        &self.core
    }

    /// Schema column overrides.
    #[must_use]
    pub fn columns(&self) -> &[ColumnTypeOverride] {
        &self.columns
    }

    /// Generated builder-local overrides.
    #[must_use]
    pub fn builders(&self) -> &[BuilderTypeOverrides] {
        &self.builders
    }
}

/// TypeScript type override for one Core type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreTypeOverride {
    core_type: crate::CoreType,
    type_override: TypeScriptTypeOverride,
}

impl CoreTypeOverride {
    /// Build a Core type override.
    #[must_use]
    pub const fn new(core_type: crate::CoreType, type_override: TypeScriptTypeOverride) -> Self {
        Self {
            core_type,
            type_override,
        }
    }

    /// Core type key being overridden.
    #[must_use]
    pub const fn core_type(&self) -> crate::CoreType {
        self.core_type
    }

    /// TypeScript type annotation override.
    #[must_use]
    pub const fn type_override(&self) -> &TypeScriptTypeOverride {
        &self.type_override
    }
}

/// TypeScript type override for one schema column.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnTypeOverride {
    reference: ColumnTypeReference,
    type_override: TypeScriptTypeOverride,
}

impl ColumnTypeOverride {
    /// Build a schema column override.
    #[must_use]
    pub const fn new(
        reference: ColumnTypeReference,
        type_override: TypeScriptTypeOverride,
    ) -> Self {
        Self {
            reference,
            type_override,
        }
    }

    /// Schema column reference being overridden.
    #[must_use]
    pub const fn reference(&self) -> &ColumnTypeReference {
        &self.reference
    }

    /// TypeScript type annotation override.
    #[must_use]
    pub const fn type_override(&self) -> &TypeScriptTypeOverride {
        &self.type_override
    }
}

/// Schema column reference accepted by `target.typescript.typeMapping.columns`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnTypeReference {
    database: Option<String>,
    table: String,
    column: String,
}

impl ColumnTypeReference {
    /// Build a schema column reference.
    #[must_use]
    pub const fn new(database: Option<String>, table: String, column: String) -> Self {
        Self {
            database,
            table,
            column,
        }
    }

    /// Optional explicit database qualifier.
    #[must_use]
    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    /// Referenced table name.
    #[must_use]
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Referenced column name.
    #[must_use]
    pub fn column(&self) -> &str {
        &self.column
    }
}

/// TypeScript type overrides scoped to one generated builder ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuilderTypeOverrides {
    builder_id: String,
    fields: Vec<NamedTypeOverride>,
    params: Vec<NamedTypeOverride>,
    repeats: Vec<RepeatTypeOverrides>,
}

impl BuilderTypeOverrides {
    /// Build builder-local type overrides.
    #[must_use]
    pub const fn new(
        builder_id: String,
        fields: Vec<NamedTypeOverride>,
        params: Vec<NamedTypeOverride>,
        repeats: Vec<RepeatTypeOverrides>,
    ) -> Self {
        Self {
            builder_id,
            fields,
            params,
            repeats,
        }
    }

    /// Generated builder ID.
    #[must_use]
    pub fn builder_id(&self) -> &str {
        &self.builder_id
    }

    /// Result field overrides.
    #[must_use]
    pub fn fields(&self) -> &[NamedTypeOverride] {
        &self.fields
    }

    /// Direct Param input overrides.
    #[must_use]
    pub fn params(&self) -> &[NamedTypeOverride] {
        &self.params
    }

    /// Repeat item field overrides.
    #[must_use]
    pub fn repeats(&self) -> &[RepeatTypeOverrides] {
        &self.repeats
    }
}

/// TypeScript type overrides scoped to one direct Repeat input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepeatTypeOverrides {
    repeat_id: String,
    fields: Vec<NamedTypeOverride>,
}

impl RepeatTypeOverrides {
    /// Build Repeat-local field overrides.
    #[must_use]
    pub const fn new(repeat_id: String, fields: Vec<NamedTypeOverride>) -> Self {
        Self { repeat_id, fields }
    }

    /// Generated Repeat input ID.
    #[must_use]
    pub fn repeat_id(&self) -> &str {
        &self.repeat_id
    }

    /// Repeat item field overrides.
    #[must_use]
    pub fn fields(&self) -> &[NamedTypeOverride] {
        &self.fields
    }
}

/// TypeScript type override keyed by a generated field or Param name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedTypeOverride {
    name: String,
    type_override: TypeScriptTypeOverride,
}

impl NamedTypeOverride {
    /// Build a named type override.
    #[must_use]
    pub const fn new(name: String, type_override: TypeScriptTypeOverride) -> Self {
        Self {
            name,
            type_override,
        }
    }

    /// Generated field, Param, or Repeat item field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// TypeScript type annotation override.
    #[must_use]
    pub const fn type_override(&self) -> &TypeScriptTypeOverride {
        &self.type_override
    }
}

/// A configured TypeScript type annotation and optional type-only import.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptTypeOverride {
    type_name: String,
    import: Option<TypeScriptTypeImport>,
}

impl TypeScriptTypeOverride {
    /// Build a TypeScript type annotation override.
    #[must_use]
    pub const fn new(type_name: String, import: Option<TypeScriptTypeImport>) -> Self {
        Self { type_name, import }
    }

    /// TypeScript type name.
    #[must_use]
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// Optional type-only import metadata.
    #[must_use]
    pub const fn import(&self) -> Option<&TypeScriptTypeImport> {
        self.import.as_ref()
    }
}

/// Type-only import metadata for a TypeScript type override.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeScriptTypeImport {
    from: String,
    name: String,
}

impl TypeScriptTypeImport {
    /// Build TypeScript type-only import metadata.
    #[must_use]
    pub const fn new(from: String, name: String) -> Self {
        Self { from, name }
    }

    /// Non-relative module specifier.
    #[must_use]
    pub fn from(&self) -> &str {
        &self.from
    }

    /// Imported type name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
