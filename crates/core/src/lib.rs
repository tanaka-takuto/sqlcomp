//! Inner domain types and language-neutral IR for `sqlay`.
//!
//! This crate is the innermost Clean Architecture boundary. It must not depend on
//! any other `sqlay-*` crate.

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
    CompiledBuilder, CompiledDynamicQuery, CompiledMutation, CompiledQuery, CompiledSlotBranch,
    CompiledSlotDefinition, CompiledSlotOccurrence, CompiledSqlSegment, CoreType, InputField,
    MutationKind, ParamBinding, ResultColumn,
};
pub use metadata::{DbMutationMetadata, DbParamUsage, DbQueryMetadata, DbResultColumn};
pub use plan::CompilationPlan;
pub use query::{
    AnalyzedMutation, AnalyzedQuery, Cardinality, FragmentMetadata, MutationId, MutationMetadata,
    ParamUsage, QueryId, QueryMetadata, RawFragment, RawMutation, RawQuery, RawSourceUnit,
    RepeatUsage, SlotUsage,
};
pub use reporting::{
    Diagnostic, DiagnosticReport, DiagnosticResult, DiagnosticSeverity, SourceLocation,
    SourcePosition, SourceRange,
};
