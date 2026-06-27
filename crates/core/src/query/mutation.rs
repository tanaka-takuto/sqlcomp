use std::path::{Path, PathBuf};

use crate::SourceLocation;

use super::{ParamUsage, RepeatUsage, SlotUsage};

/// Mutation identifier exactly as written in source metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationId(String);

impl MutationId {
    /// Build a mutation identifier.
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    /// Mutation ID text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Metadata parsed from a `type: mutation` annotation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationMetadata {
    id: String,
}

impl MutationMetadata {
    /// Build parsed mutation metadata.
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self { id }
    }

    /// Mutation ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Raw mutation extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawMutation {
    metadata: MutationMetadata,
    sql: String,
    analysis_sql: String,
    param_usages: Vec<ParamUsage>,
    slot_usages: Vec<SlotUsage>,
    repeat_usages: Vec<RepeatUsage>,
    source_path: Option<PathBuf>,
    source_location: Option<SourceLocation>,
}

impl RawMutation {
    /// Build a raw mutation from parsed metadata and SQL text.
    #[must_use]
    pub fn new(metadata: MutationMetadata, sql: String) -> Self {
        let analysis_sql = sql.clone();

        Self {
            metadata,
            sql,
            analysis_sql,
            param_usages: Vec::new(),
            slot_usages: Vec::new(),
            repeat_usages: Vec::new(),
            source_path: None,
            source_location: None,
        }
    }

    /// Attach SQL text used by downstream mutation analysis, metadata, and generation.
    #[must_use]
    pub fn with_analysis_sql(mut self, sql: String) -> Self {
        self.analysis_sql = sql;
        self
    }

    /// Attach inline Param usage occurrences in source order.
    #[must_use]
    pub fn with_param_usages(mut self, usages: Vec<ParamUsage>) -> Self {
        self.param_usages = usages;
        self
    }

    /// Attach inline Slot usage occurrences in source order.
    #[must_use]
    pub fn with_slot_usages(mut self, usages: Vec<SlotUsage>) -> Self {
        self.slot_usages = usages;
        self
    }

    /// Attach inline Repeat usage occurrences in source order.
    #[must_use]
    pub fn with_repeat_usages(mut self, usages: Vec<RepeatUsage>) -> Self {
        self.repeat_usages = usages;
        self
    }

    /// Attach the source SQL path relative to the configuration directory.
    #[must_use]
    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Attach source location context for diagnostics.
    #[must_use]
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = Some(location);
        self
    }

    /// Mutation metadata parsed from the source annotation.
    #[must_use]
    pub const fn metadata(&self) -> &MutationMetadata {
        &self.metadata
    }

    /// Raw SQL body for this mutation.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// SQL text for mutation analysis, metadata lookup, and generated output.
    #[must_use]
    pub fn analysis_sql(&self) -> &str {
        &self.analysis_sql
    }

    /// Inline Param occurrences in source order.
    #[must_use]
    pub fn param_usages(&self) -> &[ParamUsage] {
        &self.param_usages
    }

    /// Inline Slot insertion points in source order.
    #[must_use]
    pub fn slot_usages(&self) -> &[SlotUsage] {
        &self.slot_usages
    }

    /// Inline Repeat occurrences in source order.
    #[must_use]
    pub fn repeat_usages(&self) -> &[RepeatUsage] {
        &self.repeat_usages
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Optional source location for the SQL body.
    #[must_use]
    pub const fn source_location(&self) -> Option<&SourceLocation> {
        self.source_location.as_ref()
    }
}
