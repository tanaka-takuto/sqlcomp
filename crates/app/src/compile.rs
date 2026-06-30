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
mod repeat_expansion;
mod repeat_inputs;
mod slot_variants;
mod variant_validation;

#[cfg(test)]
mod tests;

use generation::generate_files;
pub use outcomes::{
    BuilderSummaryCounts, CheckOutcome, CompileOutcome, MutationSummary, QuerySummary,
};

/// Behavior for runs where `source.include` resolves to no SQL files.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmptySourceSetPolicy {
    /// Emit a warning and continue.
    Warn,
    /// Return an error before writing or cleaning generated files.
    Fail,
}

/// Behavior for `compile --clean` when `source.include` resolves to no SQL files.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmptySourceSetCleanPolicy {
    /// Skip stale generated file cleanup when the source set is empty.
    Skip,
    /// Allow stale generated file cleanup even when the source set is empty.
    Allow,
}

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
        Self::check_with_empty_source_policy(config, pipeline, EmptySourceSetPolicy::Warn)
    }

    /// Run the `check` command with explicit empty source-set handling.
    ///
    /// Returns non-fatal diagnostics that should be shown to the user.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, core compilation, target generation, or empty source-set policy
    /// enforcement fails.
    pub fn check_with_empty_source_policy<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
        empty_source_policy: EmptySourceSetPolicy,
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
        enforce_empty_source_policy(&plan, &output, empty_source_policy)?;

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
        Self::compile_with_empty_source_policy(config, pipeline, clean, EmptySourceSetPolicy::Warn)
    }

    /// Run the `compile` command with explicit empty source-set handling.
    ///
    /// Returns a success outcome with non-fatal diagnostics and write counts.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, generation, empty source-set policy enforcement, file writing, or
    /// stale file cleaning fails.
    pub fn compile_with_empty_source_policy<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
        clean: bool,
        empty_source_policy: EmptySourceSetPolicy,
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
        Self::compile_with_empty_source_and_clean_policies(
            config,
            pipeline,
            clean,
            empty_source_policy,
            EmptySourceSetCleanPolicy::Skip,
        )
    }

    /// Run the `compile` command with explicit empty source-set and cleanup
    /// handling.
    ///
    /// Returns a success outcome with non-fatal diagnostics and write counts.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when planning, source intake, analysis, metadata
    /// lookup, generation, empty source-set policy enforcement, file writing, or
    /// stale file cleaning fails.
    pub fn compile_with_empty_source_and_clean_policies<P, S, D, M, Q, T, W>(
        config: &core::ProjectConfig,
        pipeline: &CompilePipeline<'_, P, S, D, M, Q, T, W>,
        clean: bool,
        empty_source_policy: EmptySourceSetPolicy,
        empty_clean_policy: EmptySourceSetCleanPolicy,
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

        let mut output = generate_files(&plan, pipeline)?;
        enforce_empty_source_policy(&plan, &output, empty_source_policy)?;
        let generated_file_paths = output
            .generated_files
            .files()
            .iter()
            .map(|file| file.path().to_path_buf())
            .collect::<Vec<_>>();
        pipeline
            .generated_file_writer
            .write(&output.generated_files)?;

        let stale_file_removal_count =
            if should_clean_stale_generated_files(clean, &mut output, empty_clean_policy) {
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

fn should_clean_stale_generated_files(
    clean: bool,
    output: &mut generation::GeneratedPipelineOutput,
    empty_clean_policy: EmptySourceSetCleanPolicy,
) -> bool {
    if !clean {
        return false;
    }

    if output.source_file_count != 0 || empty_clean_policy == EmptySourceSetCleanPolicy::Allow {
        return true;
    }

    output.diagnostics.push(core::Diagnostic::warning(
        "skipped stale generated file cleanup because no SQL files matched",
    ));
    false
}

fn enforce_empty_source_policy(
    plan: &core::CompilationPlan,
    output: &generation::GeneratedPipelineOutput,
    policy: EmptySourceSetPolicy,
) -> core::DiagnosticResult<()> {
    if output.source_file_count != 0 || policy == EmptySourceSetPolicy::Warn {
        return Ok(());
    }

    Err(core::DiagnosticReport::new(core::Diagnostic::error(
        generation::empty_source_set_error_message(plan),
    )))
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
