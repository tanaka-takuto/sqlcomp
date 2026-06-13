//! CLI driver boundary.
//!
//! The CLI is the composition root. It wires application ports to concrete
//! adapters and is the only crate that should depend on all adapter crates.

use std::process::ExitCode;

use sqlcomp_adapters::config_jsonc::JsoncConfigLoader;
use sqlcomp_adapters::dialect_mysql::MysqlDialectAnalyzer;
use sqlcomp_adapters::metadata_mysql_sqlx::SqlxMysqlMetadataProvider;
use sqlcomp_adapters::output_fs::FileSystemGeneratedFileWriter;
use sqlcomp_adapters::source_fs::FileSystemSourceReader;
use sqlcomp_adapters::target_typescript::TypeScriptTargetGenerator;
use sqlcomp_app::{self as app, DefaultCompilationPlanner, DefaultQueryCompiler};

/// Default CLI composition root.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultPipeline;

impl app::CompileUseCasePorts for DefaultPipeline {
    type ConfigLoader = JsoncConfigLoader;
    type CompilationPlanner = DefaultCompilationPlanner;
    type SourceReader = FileSystemSourceReader;
    type DialectAnalyzer = MysqlDialectAnalyzer;
    type MetadataProvider = SqlxMysqlMetadataProvider;
    type QueryCompiler = DefaultQueryCompiler;
    type TargetGenerator = TypeScriptTargetGenerator;
    type GeneratedFileWriter = FileSystemGeneratedFileWriter;
}

/// Run the `sqlcomp` command-line interface.
#[must_use]
pub const fn run() -> ExitCode {
    ExitCode::SUCCESS
}
