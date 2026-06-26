use std::path::Path;

use sqlay_core as core;

/// Port for creating a starter project configuration file.
pub trait ConfigTemplateWriter {
    /// Write starter configuration content to a new file.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when the target file already exists or cannot be
    /// written.
    fn write_new(&self, path: &Path, contents: &str) -> core::DiagnosticResult<()>;
}

/// Port for loading project configuration.
pub trait ConfigLoader {
    /// Load and validate project configuration.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when configuration cannot be found, parsed, or
    /// validated.
    fn load(&self) -> core::DiagnosticResult<core::ProjectConfig>;
}

/// Application service for constructing compilation plans.
pub trait CompilationPlanner {
    /// Convert project configuration into a resolved compilation plan.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when source, output, database, or target settings
    /// cannot be resolved into an executable plan.
    fn plan(&self, config: &core::ProjectConfig) -> core::DiagnosticResult<core::CompilationPlan>;
}

/// Port for reading SQL source files.
pub trait SourceReader {
    /// Read source files described by the compilation plan.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when source files cannot be discovered, read, or
    /// converted into raw query blocks.
    fn read(&self, plan: &core::CompilationPlan) -> core::DiagnosticResult<SourceRead>;
}

/// Source intake output, including non-fatal diagnostics discovered while
/// reading included SQL files.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceRead {
    queries: Vec<core::RawQuery>,
    mutations: Vec<core::RawMutation>,
    fragments: Vec<core::RawFragment>,
    source_units: Vec<core::RawSourceUnit>,
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
}

impl SourceRead {
    /// Build source intake output.
    #[must_use]
    pub const fn new(queries: Vec<core::RawQuery>, diagnostics: core::DiagnosticReport) -> Self {
        Self {
            queries,
            mutations: Vec::new(),
            fragments: Vec::new(),
            source_units: Vec::new(),
            diagnostics,
            source_file_count: 0,
        }
    }

    /// Build source intake output without diagnostics.
    #[must_use]
    pub const fn from_queries(queries: Vec<core::RawQuery>) -> Self {
        Self::new(
            queries,
            core::DiagnosticReport::from_diagnostics(Vec::new()),
        )
    }

    /// Attach the number of SQL files matched by source discovery.
    #[must_use]
    pub const fn with_source_file_count(mut self, source_file_count: usize) -> Self {
        self.source_file_count = source_file_count;
        self
    }

    /// Attach global fragment source units found in included SQL sources.
    #[must_use]
    pub fn with_fragments(mut self, fragments: Vec<core::RawFragment>) -> Self {
        self.fragments = fragments;
        self
    }

    /// Attach global mutation source units found in included SQL sources.
    #[must_use]
    pub fn with_mutations(mut self, mutations: Vec<core::RawMutation>) -> Self {
        self.mutations = mutations;
        self
    }

    /// Attach top-level source units in source order.
    #[must_use]
    pub fn with_source_units(mut self, source_units: Vec<core::RawSourceUnit>) -> Self {
        self.source_units = source_units;
        self
    }

    /// Query blocks found in included SQL sources.
    #[must_use]
    pub fn queries(&self) -> &[core::RawQuery] {
        &self.queries
    }

    /// Mutation blocks found in included SQL sources.
    #[must_use]
    pub fn mutations(&self) -> &[core::RawMutation] {
        &self.mutations
    }

    /// Fragment blocks found in included SQL sources.
    #[must_use]
    pub fn fragments(&self) -> &[core::RawFragment] {
        &self.fragments
    }

    /// Top-level source units found in included SQL sources, preserving source order.
    #[must_use]
    pub fn source_units(&self) -> &[core::RawSourceUnit] {
        &self.source_units
    }

    /// Non-fatal diagnostics found during source intake.
    #[must_use]
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        &self.diagnostics
    }

    /// Number of SQL source files matched by the compilation plan.
    #[must_use]
    pub const fn source_file_count(&self) -> usize {
        self.source_file_count
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        Vec<core::RawQuery>,
        Vec<core::RawMutation>,
        Vec<core::RawFragment>,
        Vec<core::RawSourceUnit>,
        core::DiagnosticReport,
    ) {
        (
            self.queries,
            self.mutations,
            self.fragments,
            self.source_units,
            self.diagnostics,
        )
    }
}

/// Port for dialect-specific SQL analysis.
pub trait DialectAnalyzer {
    /// Analyze one raw query.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when SQL is invalid for the configured dialect or
    /// outside the supported statement shape.
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery>;
}

/// Port for dialect-specific mutation SQL analysis.
pub trait MutationAnalyzer {
    /// Analyze one raw mutation.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when SQL is invalid for the configured dialect or
    /// outside the supported mutation statement shape.
    fn analyze_mutation(
        &self,
        mutation: &core::RawMutation,
    ) -> core::DiagnosticResult<core::AnalyzedMutation>;
}

/// Port for database-backed metadata lookup.
pub trait MetadataProvider {
    /// Describe database metadata for one analyzed query.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when metadata lookup cannot connect to the database
    /// or describe the analyzed query.
    fn describe(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata>;
}

/// Port for schema-backed mutation metadata lookup.
pub trait MutationMetadataProvider {
    /// Describe database metadata for one analyzed mutation without executing it.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when schema metadata lookup cannot connect to the
    /// database or resolve mutation Param metadata.
    fn describe_mutation(
        &self,
        mutation: &core::RawMutation,
        analysis: &core::AnalyzedMutation,
    ) -> core::DiagnosticResult<core::DbMutationMetadata>;
}

/// Application service for compiling analyzed queries into core IR.
pub trait QueryCompiler {
    /// Compile one analyzed query into language-neutral IR.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when analyzed query facts and database metadata cannot
    /// be converted into the core IR.
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery>;
}

/// Application service for compiling analyzed mutations into core IR.
pub trait MutationCompiler {
    /// Compile one analyzed mutation into language-neutral IR.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when analyzed mutation facts and database metadata
    /// cannot be converted into the core IR.
    fn compile_mutation(
        &self,
        mutation: &core::RawMutation,
        analysis: &core::AnalyzedMutation,
        metadata: &core::DbMutationMetadata,
    ) -> core::DiagnosticResult<core::CompiledMutation>;
}

/// Port for target-language generation.
pub trait TargetGenerator {
    /// Generate target files from compiled builders.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when target-language files cannot be generated from
    /// core IR.
    fn generate(
        &self,
        plan: &core::CompilationPlan,
        builders: &[core::CompiledBuilder],
    ) -> core::DiagnosticResult<core::GeneratedFiles>;
}

/// Port for writing generated files.
pub trait GeneratedFileWriter {
    /// Persist generated files.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when generated files cannot be written.
    fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()>;
}

/// Port for removing stale managed generated files.
pub trait GeneratedFileCleaner {
    /// Remove generated files under `output_dir` that are managed by sqlay and
    /// not present in `current_files`.
    ///
    /// Returns the number of stale generated files removed.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when generated files cannot be inspected or removed.
    fn clean_stale(
        &self,
        output_dir: &Path,
        current_files: &core::GeneratedFiles,
    ) -> core::DiagnosticResult<usize>;
}
