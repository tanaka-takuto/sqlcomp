use std::path::{Path, PathBuf};

use sqlcomp_core as core;

use crate::{
    CompilationPlanner, DialectAnalyzer, GeneratedFileCleaner, GeneratedFileWriter,
    MetadataProvider, QueryCompiler, SourceReader, TargetGenerator,
};

mod diagnostics;
mod dynamic_ir;
mod generation;
mod param_validation;
mod slot_variants;
mod variant_validation;

#[cfg(test)]
mod tests;

use generation::generate_files;

/// Application service for compile-like CLI commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompileUseCase;

/// Successful `check` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckOutcome {
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    fragment_count: usize,
}

impl CheckOutcome {
    /// Build a successful check outcome.
    #[must_use]
    pub const fn new(
        diagnostics: core::DiagnosticReport,
        source_file_count: usize,
        output_dir: PathBuf,
        query_summaries: Vec<QuerySummary>,
        fragment_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            fragment_count,
        }
    }

    /// Non-fatal diagnostics that should be shown to the user.
    #[must_use]
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        &self.diagnostics
    }

    /// Number of SQL source files matched by source discovery.
    #[must_use]
    pub const fn source_file_count(&self) -> usize {
        self.source_file_count
    }

    /// Number of query blocks compiled.
    #[must_use]
    pub const fn query_count(&self) -> usize {
        self.query_summaries.len()
    }

    /// Generated output directory for this run.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Query-level summary data in source order.
    #[must_use]
    pub fn query_summaries(&self) -> &[QuerySummary] {
        &self.query_summaries
    }

    /// Number of global Fragment source units resolved in this run.
    #[must_use]
    pub const fn fragment_count(&self) -> usize {
        self.fragment_count
    }

    /// Number of unique query-local Slots resolved across compiled queries.
    #[must_use]
    pub fn unique_slot_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::slot_count)
            .sum()
    }

    /// Number of SQL variants validated across compiled queries.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::variant_count)
            .sum()
    }
}

/// Successful `compile` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileOutcome {
    diagnostics: core::DiagnosticReport,
    source_file_count: usize,
    output_dir: PathBuf,
    query_summaries: Vec<QuerySummary>,
    generated_file_paths: Vec<PathBuf>,
    stale_file_removal_count: Option<usize>,
    fragment_count: usize,
}

impl CompileOutcome {
    /// Build a successful compile outcome.
    #[must_use]
    pub const fn new(
        diagnostics: core::DiagnosticReport,
        source_file_count: usize,
        output_dir: PathBuf,
        query_summaries: Vec<QuerySummary>,
        generated_file_paths: Vec<PathBuf>,
        stale_file_removal_count: Option<usize>,
        fragment_count: usize,
    ) -> Self {
        Self {
            diagnostics,
            source_file_count,
            output_dir,
            query_summaries,
            generated_file_paths,
            stale_file_removal_count,
            fragment_count,
        }
    }

    /// Non-fatal diagnostics that should be shown to the user.
    #[must_use]
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        &self.diagnostics
    }

    /// Number of SQL source files matched by source discovery.
    #[must_use]
    pub const fn source_file_count(&self) -> usize {
        self.source_file_count
    }

    /// Number of query blocks compiled.
    #[must_use]
    pub const fn query_count(&self) -> usize {
        self.query_summaries.len()
    }

    /// Generated output directory for this run.
    #[must_use]
    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Query-level summary data in source order.
    #[must_use]
    pub fn query_summaries(&self) -> &[QuerySummary] {
        &self.query_summaries
    }

    /// Number of generated files written or updated.
    #[must_use]
    pub const fn generated_file_count(&self) -> usize {
        self.generated_file_paths.len()
    }

    /// Generated file paths written or updated by this run.
    #[must_use]
    pub fn generated_file_paths(&self) -> &[PathBuf] {
        &self.generated_file_paths
    }

    /// Number of stale generated files removed when cleanup ran.
    #[must_use]
    pub const fn stale_file_removal_count(&self) -> Option<usize> {
        self.stale_file_removal_count
    }

    /// Number of global Fragment source units resolved in this run.
    #[must_use]
    pub const fn fragment_count(&self) -> usize {
        self.fragment_count
    }

    /// Number of unique query-local Slots resolved across compiled queries.
    #[must_use]
    pub fn unique_slot_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::slot_count)
            .sum()
    }

    /// Number of SQL variants validated across compiled queries.
    #[must_use]
    pub fn variant_count(&self) -> usize {
        self.query_summaries
            .iter()
            .map(QuerySummary::variant_count)
            .sum()
    }
}

/// Query-level success summary data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySummary {
    id: String,
    source_path: Option<PathBuf>,
    param_count: usize,
    input_field_count: usize,
    slot_count: usize,
    variant_count: usize,
}

impl QuerySummary {
    /// Build query-level summary data.
    #[must_use]
    pub const fn new(
        id: String,
        source_path: Option<PathBuf>,
        param_count: usize,
        input_field_count: usize,
        slot_count: usize,
        variant_count: usize,
    ) -> Self {
        Self {
            id,
            source_path,
            param_count,
            input_field_count,
            slot_count,
            variant_count,
        }
    }

    /// Query ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Number of generated parameter bindings, matching SQL placeholder occurrences.
    #[must_use]
    pub const fn param_count(&self) -> usize {
        self.param_count
    }

    /// Number of public input fields generated for this query.
    #[must_use]
    pub const fn input_field_count(&self) -> usize {
        self.input_field_count
    }

    /// Number of unique query-local Slots resolved for this query.
    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Number of SQL variants validated for this query.
    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.variant_count
    }

    fn from_compiled_query(
        query: &core::CompiledQuery,
        slot_count: usize,
        variant_count: usize,
    ) -> Self {
        Self::new(
            query.id().as_str().to_owned(),
            query.source_path().map(Path::to_path_buf),
            query.params().len(),
            query.input().len(),
            slot_count,
            variant_count,
        )
    }
}

/// Concrete port references required to run the compile pipeline.
#[derive(Clone, Copy, Debug)]
pub struct CompilePipeline<'a, P, S, D, M, Q, T, W>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer,
    M: MetadataProvider,
    Q: QueryCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    /// Compilation planner implementation.
    pub planner: &'a P,
    /// SQL source reader implementation.
    pub source_reader: &'a S,
    /// Dialect analyzer implementation.
    pub dialect_analyzer: &'a D,
    /// Database metadata provider implementation.
    pub metadata_provider: &'a M,
    /// Core IR compiler implementation.
    pub query_compiler: &'a Q,
    /// Target-language generator implementation.
    pub target_generator: &'a T,
    /// Generated file writer and cleaner implementation.
    pub generated_file_writer: &'a W,
}

impl DefaultCompileUseCase {
    /// Run the `check` command as a dry run of the full generation pipeline.
    ///
    /// Returns non-fatal diagnostics that should be shown to the user.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, core compilation, or target generation fails.
    pub fn check<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
    ) -> core::DiagnosticResult<CheckOutcome>
    where
        P: CompilationPlanner,
        S: SourceReader,
        D: DialectAnalyzer,
        M: MetadataProvider,
        Q: QueryCompiler,
        T: TargetGenerator,
        W: GeneratedFileWriter,
    {
        let plan = pipeline.planner.plan(config)?;
        let output = generate_files(&plan, pipeline)?;

        Ok(CheckOutcome::new(
            output.diagnostics,
            output.source_file_count,
            output.output_dir,
            output.query_summaries,
            output.fragment_count,
        ))
    }

    /// Run the `compile` command.
    ///
    /// Returns a success outcome with non-fatal diagnostics and write counts.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, generation, or file writing fails.
    pub fn compile<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
        clean: bool,
    ) -> core::DiagnosticResult<CompileOutcome>
    where
        P: CompilationPlanner,
        S: SourceReader,
        D: DialectAnalyzer,
        M: MetadataProvider,
        Q: QueryCompiler,
        T: TargetGenerator,
        W: GeneratedFileWriter + GeneratedFileCleaner,
    {
        let plan = pipeline.planner.plan(config)?;

        let output = generate_files(&plan, pipeline)?;
        let generated_file_paths = output
            .generated_files
            .files()
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect::<Vec<_>>();
        pipeline
            .generated_file_writer
            .write(&output.generated_files)?;

        let stale_file_removal_count = if clean {
            Some(
                pipeline
                    .generated_file_writer
                    .clean_stale(plan.output_dir(), &output.generated_files)?,
            )
        } else {
            None
        };

        Ok(CompileOutcome::new(
            output.diagnostics,
            output.source_file_count,
            output.output_dir,
            output.query_summaries,
            generated_file_paths,
            stale_file_removal_count,
            output.fragment_count,
        ))
    }
}

/// Dummy port bundle showing dependencies required by compile-like use cases.
pub trait CompileUseCasePorts {
    /// Configuration loader implementation.
    type ConfigLoader: crate::ConfigLoader;

    /// Compilation planner implementation.
    type CompilationPlanner: CompilationPlanner;

    /// Source reader implementation.
    type SourceReader: SourceReader;

    /// Dialect analyzer implementation.
    type DialectAnalyzer: DialectAnalyzer;

    /// Metadata provider implementation.
    type MetadataProvider: MetadataProvider;

    /// Query compiler implementation.
    type QueryCompiler: QueryCompiler;

    /// Target generator implementation.
    type TargetGenerator: TargetGenerator;

    /// Generated file writer and cleaner implementation.
    type GeneratedFileWriter: GeneratedFileWriter + GeneratedFileCleaner;
}
