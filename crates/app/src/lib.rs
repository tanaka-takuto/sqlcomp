//! Application use cases and ports.
//!
//! This crate depends only on `sqlcomp-core`. Adapter crates implement these
//! ports; `sqlcomp-app` must not depend on concrete adapters.

mod compile;
mod constants;
mod init;
mod planning;
mod ports;
mod query_compiler;

#[cfg(test)]
mod tests;

pub use compile::{
    CheckOutcome, CompileOutcome, CompilePipeline, CompileUseCasePorts, DefaultCompileUseCase,
    QuerySummary,
};
pub use constants::{CONFIG_FILE_NAME, STARTER_CONFIG_TEMPLATE};
pub use init::DefaultProjectInitializer;
pub use planning::DefaultCompilationPlanner;
pub use ports::{
    CompilationPlanner, ConfigLoader, ConfigTemplateWriter, DialectAnalyzer, GeneratedFileCleaner,
    GeneratedFileWriter, MetadataProvider, QueryCompiler, SourceRead, SourceReader,
    TargetGenerator,
};
pub use query_compiler::DefaultQueryCompiler;
