//! Application use cases and ports.
//!
//! This crate depends only on `sqlcomp-core`. Adapter crates implement these
//! ports; `sqlcomp-app` must not depend on concrete adapters.

use sqlcomp_core as core;

/// Port for loading project configuration.
pub trait ConfigLoader {
    /// Load and validate project configuration.
    fn load(&self) -> core::ProjectConfig;
}

/// Application service for constructing compilation plans.
pub trait CompilationPlanner {
    /// Convert project configuration into a resolved compilation plan.
    fn plan(&self, config: &core::ProjectConfig) -> core::CompilationPlan;
}

/// Port for reading SQL source files.
pub trait SourceReader {
    /// Read source files described by the compilation plan.
    fn read(&self, plan: &core::CompilationPlan) -> Vec<core::RawQuery>;
}

/// Port for dialect-specific SQL analysis.
pub trait DialectAnalyzer {
    /// Analyze one raw query.
    fn analyze(&self, query: &core::RawQuery) -> core::AnalyzedQuery;
}

/// Port for database-backed metadata lookup.
pub trait MetadataProvider {
    /// Describe database metadata for one analyzed query.
    fn describe(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
    ) -> core::DbQueryMetadata;
}

/// Application service for compiling analyzed queries into core IR.
pub trait QueryCompiler {
    /// Compile one analyzed query into language-neutral IR.
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::CompiledQuery;
}

/// Port for target-language generation.
pub trait TargetGenerator {
    /// Generate target files from compiled queries.
    fn generate(&self, queries: &[core::CompiledQuery]) -> core::GeneratedFiles;
}

/// Port for writing generated files.
pub trait GeneratedFileWriter {
    /// Persist generated files.
    fn write(&self, files: &core::GeneratedFiles);
}

/// Dummy port bundle showing dependencies required by compile-like use cases.
pub trait CompileUseCasePorts {
    /// Configuration loader implementation.
    type ConfigLoader: ConfigLoader;

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

    /// Generated file writer implementation.
    type GeneratedFileWriter: GeneratedFileWriter;
}

/// Default application-owned compilation planner.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultCompilationPlanner;

impl CompilationPlanner for DefaultCompilationPlanner {
    fn plan(&self, _config: &core::ProjectConfig) -> core::CompilationPlan {
        core::CompilationPlan
    }
}

/// Default application-owned query compiler.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultQueryCompiler;

impl QueryCompiler for DefaultQueryCompiler {
    fn compile(
        &self,
        _query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
        _metadata: &core::DbQueryMetadata,
    ) -> core::CompiledQuery {
        core::CompiledQuery
    }
}
