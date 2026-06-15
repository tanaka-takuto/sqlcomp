use sqlcomp_core as core;

use crate::{
    CompilationPlanner, DialectAnalyzer, GeneratedFileCleaner, GeneratedFileWriter,
    MetadataProvider, QueryCompiler, SourceReader, TargetGenerator,
};

/// Application service for compile-like CLI commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompileUseCase;

/// Successful `compile` command outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileOutcome {
    diagnostics: core::DiagnosticReport,
    generated_file_count: usize,
    stale_file_removal_count: Option<usize>,
}

impl CompileOutcome {
    /// Build a successful compile outcome.
    #[must_use]
    pub const fn new(
        diagnostics: core::DiagnosticReport,
        generated_file_count: usize,
        stale_file_removal_count: Option<usize>,
    ) -> Self {
        Self {
            diagnostics,
            generated_file_count,
            stale_file_removal_count,
        }
    }

    /// Non-fatal diagnostics that should be shown to the user.
    #[must_use]
    pub const fn diagnostics(&self) -> &core::DiagnosticReport {
        &self.diagnostics
    }

    /// Number of generated files written or updated.
    #[must_use]
    pub const fn generated_file_count(&self) -> usize {
        self.generated_file_count
    }

    /// Number of stale generated files removed when cleanup ran.
    #[must_use]
    pub const fn stale_file_removal_count(&self) -> Option<usize> {
        self.stale_file_removal_count
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
    ) -> core::DiagnosticResult<core::DiagnosticReport>
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

        Ok(output.diagnostics)
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
        let generated_file_count = output.generated_files.files().len();
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
            generated_file_count,
            stale_file_removal_count,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedPipelineOutput {
    generated_files: core::GeneratedFiles,
    diagnostics: core::DiagnosticReport,
}

fn generate_files<P, S, D, M, Q, T, W>(
    plan: &core::CompilationPlan,
    pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
) -> core::DiagnosticResult<GeneratedPipelineOutput>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer,
    M: MetadataProvider,
    Q: QueryCompiler,
    T: TargetGenerator,
    W: GeneratedFileWriter,
{
    let (raw_queries, diagnostics) = pipeline.source_reader.read(plan)?.into_parts();
    let mut compiled_queries = Vec::with_capacity(raw_queries.len());

    for query in &raw_queries {
        let analysis = pipeline.dialect_analyzer.analyze(query)?;
        let metadata = pipeline.metadata_provider.describe(query, &analysis)?;
        let compiled = pipeline
            .query_compiler
            .compile(query, &analysis, &metadata)?;
        compiled_queries.push(compiled);
    }

    let generated_files = pipeline
        .target_generator
        .generate(plan, &compiled_queries)?;

    Ok(GeneratedPipelineOutput {
        generated_files,
        diagnostics,
    })
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
