use sqlay_core as core;

use crate::{
    CompilationPlanner, DialectAnalyzer, GeneratedFileCleaner, GeneratedFileWriter,
    MetadataProvider, MutationAnalyzer, MutationCompiler, MutationMetadataProvider, QueryCompiler,
    SourceReader, TargetGenerator,
};

mod diagnostics;
mod dynamic_ir;
mod generation;
mod outcomes;
mod param_validation;
mod repeat_inputs;
mod slot_variants;
mod variant_validation;

#[cfg(test)]
mod tests;

use generation::generate_files;
pub use outcomes::{CheckOutcome, CompileOutcome, MutationSummary, QuerySummary};

/// Application service for compile-like CLI commands.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompileUseCase;

/// Concrete port references required to run the compile pipeline.
#[derive(Clone, Copy, Debug)]
pub struct CompilePipeline<'a, P, S, D, M, Q, T, W>
where
    P: CompilationPlanner,
    S: SourceReader,
    D: DialectAnalyzer + MutationAnalyzer,
    M: MetadataProvider + MutationMetadataProvider,
    Q: QueryCompiler + MutationCompiler,
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
        D: DialectAnalyzer + MutationAnalyzer,
        M: MetadataProvider + MutationMetadataProvider,
        Q: QueryCompiler + MutationCompiler,
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
            output.mutation_summaries,
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
        D: DialectAnalyzer + MutationAnalyzer,
        M: MetadataProvider + MutationMetadataProvider,
        Q: QueryCompiler + MutationCompiler,
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
            output.mutation_summaries,
            generated_file_paths,
            output.fragment_count,
        )
        .with_stale_file_removal_count(stale_file_removal_count))
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
    type DialectAnalyzer: DialectAnalyzer + MutationAnalyzer;

    /// Metadata provider implementation.
    type MetadataProvider: MetadataProvider + MutationMetadataProvider;

    /// Query compiler implementation.
    type QueryCompiler: QueryCompiler + MutationCompiler;

    /// Target generator implementation.
    type TargetGenerator: TargetGenerator;

    /// Generated file writer and cleaner implementation.
    type GeneratedFileWriter: GeneratedFileWriter + GeneratedFileCleaner;
}
