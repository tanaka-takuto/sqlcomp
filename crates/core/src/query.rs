use std::path::{Path, PathBuf};

use crate::{CoreType, SourceLocation};

mod mutation;

pub use mutation::{MutationId, MutationMetadata, RawMutation};

/// Query identifier exactly as written in source metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryId(String);

impl QueryId {
    /// Build a query identifier.
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    /// Query ID text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Metadata parsed from a `type: query` annotation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryMetadata {
    id: String,
    cardinality: Option<Cardinality>,
}

impl QueryMetadata {
    /// Build parsed query metadata.
    #[must_use]
    pub const fn new(id: String, cardinality: Option<Cardinality>) -> Self {
        Self { id, cardinality }
    }

    /// Query ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Optional cardinality override from source metadata.
    #[must_use]
    pub const fn cardinality(&self) -> Option<Cardinality> {
        self.cardinality
    }
}

/// Metadata parsed from a `type: fragment` annotation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FragmentMetadata {
    id: String,
}

impl FragmentMetadata {
    /// Build parsed fragment metadata.
    #[must_use]
    pub const fn new(id: String) -> Self {
        Self { id }
    }

    /// Fragment ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// One inline Param occurrence in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamUsage {
    id: String,
    value_type_override: Option<CoreType>,
    nullable_override: bool,
    sample_sql: String,
    placeholder_index: Option<usize>,
    source_location: SourceLocation,
}

impl ParamUsage {
    /// Build a Param usage occurrence.
    #[must_use]
    pub const fn new(
        id: String,
        value_type_override: Option<CoreType>,
        nullable_override: bool,
        source_location: SourceLocation,
    ) -> Self {
        Self {
            id,
            value_type_override,
            nullable_override,
            sample_sql: String::new(),
            placeholder_index: None,
            source_location,
        }
    }

    /// Attach the source sample expression covered by this Param range.
    #[must_use]
    pub fn with_sample_sql(mut self, sample_sql: String) -> Self {
        self.sample_sql = sample_sql;
        self
    }

    /// Attach the byte index of this Param's generated `?` in `analysis_sql`.
    #[must_use]
    pub const fn with_placeholder_index(mut self, index: usize) -> Self {
        self.placeholder_index = Some(index);
        self
    }

    /// Param ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Inline `valueType` override, when provided.
    #[must_use]
    pub const fn value_type_override(&self) -> Option<CoreType> {
        self.value_type_override
    }

    /// Whether the inline marker declared `nullable: true`.
    #[must_use]
    pub const fn nullable_override(&self) -> bool {
        self.nullable_override
    }

    /// Source sample expression between `param` and `paramEnd`.
    #[must_use]
    pub fn sample_sql(&self) -> &str {
        &self.sample_sql
    }

    /// Byte index of this Param's generated `?` in the owning analysis SQL.
    #[must_use]
    pub const fn placeholder_index(&self) -> Option<usize> {
        self.placeholder_index
    }

    /// Source location for the Param range.
    #[must_use]
    pub const fn source_location(&self) -> &SourceLocation {
        &self.source_location
    }

    /// Replace source location context.
    #[must_use]
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = location;
        self
    }
}

/// One inline Slot insertion point in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotUsage {
    id: String,
    targets: Vec<String>,
    insertion_index: usize,
    source_location: SourceLocation,
}

impl SlotUsage {
    /// Build a Slot usage occurrence.
    #[must_use]
    pub const fn new(
        id: String,
        targets: Vec<String>,
        insertion_index: usize,
        source_location: SourceLocation,
    ) -> Self {
        Self {
            id,
            targets,
            insertion_index,
            source_location,
        }
    }

    /// Slot ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Target fragment IDs exactly as written in source metadata.
    #[must_use]
    pub fn targets(&self) -> &[String] {
        &self.targets
    }

    /// Byte index in `RawQuery::analysis_sql()` where the slot marker was removed.
    ///
    /// `RawQuery::slot_usages()` must expose Slot usages in ascending insertion-index
    /// order. Application Slot expansion validates and rebuilds SQL with a cursor that
    /// relies on this ordering contract from source intake.
    #[must_use]
    pub const fn insertion_index(&self) -> usize {
        self.insertion_index
    }

    /// Source location for the Slot marker.
    #[must_use]
    pub const fn source_location(&self) -> &SourceLocation {
        &self.source_location
    }

    /// Replace source location context.
    #[must_use]
    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = location;
        self
    }
}

/// Raw fragment extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawFragment {
    metadata: FragmentMetadata,
    sql: String,
    analysis_sql: String,
    param_usages: Vec<ParamUsage>,
    source_path: Option<PathBuf>,
    source_location: Option<SourceLocation>,
}

impl RawFragment {
    /// Build a raw fragment from parsed metadata and SQL text.
    #[must_use]
    pub fn new(metadata: FragmentMetadata, sql: String) -> Self {
        let analysis_sql = sql.clone();

        Self {
            metadata,
            sql,
            analysis_sql,
            param_usages: Vec::new(),
            source_path: None,
            source_location: None,
        }
    }

    /// Attach SQL text used when the fragment is inserted into a query variant.
    #[must_use]
    pub fn with_analysis_sql(mut self, sql: String) -> Self {
        self.analysis_sql = sql;
        self
    }

    /// Attach inline Param usage occurrences in fragment source order.
    #[must_use]
    pub fn with_param_usages(mut self, usages: Vec<ParamUsage>) -> Self {
        self.param_usages = usages;
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

    /// Fragment metadata parsed from the source annotation.
    #[must_use]
    pub const fn metadata(&self) -> &FragmentMetadata {
        &self.metadata
    }

    /// Raw SQL body for this fragment.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// SQL text used when this fragment is inserted into a query variant.
    #[must_use]
    pub fn analysis_sql(&self) -> &str {
        &self.analysis_sql
    }

    /// Inline Param occurrences in fragment source order.
    #[must_use]
    pub fn param_usages(&self) -> &[ParamUsage] {
        &self.param_usages
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

/// Raw query extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawQuery {
    metadata: QueryMetadata,
    sql: String,
    analysis_sql: String,
    param_usages: Vec<ParamUsage>,
    slot_usages: Vec<SlotUsage>,
    source_path: Option<PathBuf>,
    source_location: Option<SourceLocation>,
}

impl RawQuery {
    /// Build a raw query from parsed metadata and SQL text.
    #[must_use]
    pub fn new(metadata: QueryMetadata, sql: String) -> Self {
        let analysis_sql = sql.clone();

        Self {
            metadata,
            sql,
            analysis_sql,
            param_usages: Vec::new(),
            slot_usages: Vec::new(),
            source_path: None,
            source_location: None,
        }
    }

    /// Attach SQL text used by downstream analysis, metadata, and generation.
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

    /// Query metadata parsed from the source annotation.
    #[must_use]
    pub const fn metadata(&self) -> &QueryMetadata {
        &self.metadata
    }

    /// Raw SQL body for this query.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// SQL text for dialect analysis, metadata lookup, and generated output.
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

/// Raw top-level source unit in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RawSourceUnit {
    /// A `type: query` builder source unit.
    Query(RawQuery),
    /// A `type: mutation` builder source unit.
    Mutation(RawMutation),
    /// A `type: fragment` reusable SQL fragment source unit.
    Fragment(RawFragment),
}

impl RawSourceUnit {
    /// Source unit ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Query(query) => query.metadata().id(),
            Self::Mutation(mutation) => mutation.metadata().id(),
            Self::Fragment(fragment) => fragment.metadata().id(),
        }
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        match self {
            Self::Query(query) => query.source_path(),
            Self::Mutation(mutation) => mutation.source_path(),
            Self::Fragment(fragment) => fragment.source_path(),
        }
    }

    /// Optional source location for this source unit body.
    #[must_use]
    pub const fn source_location(&self) -> Option<&SourceLocation> {
        match self {
            Self::Query(query) => query.source_location(),
            Self::Mutation(mutation) => mutation.source_location(),
            Self::Fragment(fragment) => fragment.source_location(),
        }
    }
}

/// Dialect analysis result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzedQuery {
    cardinality: Cardinality,
}

impl AnalyzedQuery {
    /// Build dialect analysis facts for a query.
    #[must_use]
    pub const fn new(cardinality: Cardinality) -> Self {
        Self { cardinality }
    }

    /// Cardinality inferred by dialect analysis.
    #[must_use]
    pub const fn cardinality(&self) -> Cardinality {
        self.cardinality
    }
}

/// Query cardinality in generated output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cardinality {
    /// A query returns zero or one row.
    One,
    /// A query returns zero or more rows.
    Many,
}
