//! Inner domain types and language-neutral IR for `sqlcomp`.
//!
//! This crate is the innermost Clean Architecture boundary. It must not depend on
//! any other `sqlcomp-*` crate.

/// Dummy project configuration accepted by application use cases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectConfig;

/// Dummy resolved compilation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompilationPlan;

/// Dummy query identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryId;

/// Dummy raw query extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawQuery;

/// Dummy dialect analysis result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzedQuery;

/// Dummy database metadata description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbQueryMetadata;

/// Dummy language-neutral compiled query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledQuery;

/// Dummy generated file set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFiles;

/// Query cardinality in generated output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cardinality {
    /// A query returns zero or one row.
    One,
    /// A query returns zero or more rows.
    Many,
}

/// Language-neutral type classification for generated output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreType {
    /// Boolean value.
    Bool,
    /// 32-bit integer value.
    Int32,
    /// 64-bit integer value.
    Int64,
    /// 64-bit floating-point value.
    Float64,
    /// Decimal value.
    Decimal,
    /// Text value.
    String,
    /// Binary value.
    Bytes,
    /// Date value.
    Date,
    /// Date-time value.
    DateTime,
    /// JSON value.
    Json,
    /// Unknown database type.
    Unknown,
}
