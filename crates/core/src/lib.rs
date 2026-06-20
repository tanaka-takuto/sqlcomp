//! Inner domain types and language-neutral IR for `sqlcomp`.
//!
//! This crate is the innermost Clean Architecture boundary. It must not depend on
//! any other `sqlcomp-*` crate.

mod config;
mod generated;
mod ir;
mod metadata;
mod plan;
mod query;
mod reporting;

pub use config::{
    DatabaseConfig, DatabaseDialect, OutputConfig, ProjectConfig, SourceConfig, TargetConfig,
    TargetLanguage,
};
pub use generated::{GENERATED_FILE_HEADER, GeneratedFile, GeneratedFiles};
pub use ir::{
    CompiledDynamicQuery, CompiledQuery, CompiledSlotBranch, CompiledSlotDefinition,
    CompiledSlotOccurrence, CompiledSqlSegment, CoreType, InputField, ParamBinding, ResultColumn,
};
pub use metadata::{DbParamUsage, DbQueryMetadata, DbResultColumn};
pub use plan::CompilationPlan;
pub use query::{
    AnalyzedQuery, Cardinality, FragmentMetadata, ParamUsage, QueryId, QueryMetadata, RawFragment,
    RawQuery, SlotUsage,
};
pub use reporting::{
    Diagnostic, DiagnosticReport, DiagnosticResult, DiagnosticSeverity, SourceLocation,
    SourcePosition, SourceRange,
};
