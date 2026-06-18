use std::path::{Path, PathBuf};

use crate::{CoreType, SourceLocation};

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
            source_location,
        }
    }

    /// Attach the source sample expression covered by this Param range.
    #[must_use]
    pub fn with_sample_sql(mut self, sample_sql: String) -> Self {
        self.sample_sql = sample_sql;
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        AnalyzedQuery, Cardinality, FragmentMetadata, QueryMetadata, RawFragment, RawQuery,
        SlotUsage, SourceLocation, SourcePosition, SourceRange,
    };

    #[test]
    fn raw_query_preserves_metadata_sql_source_path_and_optional_source_location() {
        let location = SourceLocation::at_range(
            "sql/users.sql",
            SourceRange::point(
                SourcePosition::one_based(8, 1).expect("test position should be valid"),
            ),
        );
        let query = RawQuery::new(
            QueryMetadata::new("listUsers".to_owned(), Some(Cardinality::One)),
            "SELECT id FROM users;".to_owned(),
        )
        .with_source_path("sql/users.sql")
        .with_source_location(location.clone());

        assert_eq!(query.metadata().id(), "listUsers");
        assert_eq!(query.metadata().cardinality(), Some(Cardinality::One));
        assert_eq!(query.sql(), "SELECT id FROM users;");
        assert_eq!(query.source_path(), Some(Path::new("sql/users.sql")));
        assert_eq!(query.source_location(), Some(&location));
    }

    #[test]
    fn raw_query_can_carry_analysis_sql_and_param_usages() {
        let location = SourceLocation::from_range(SourceRange::point(
            SourcePosition::one_based(8, 15).expect("test position should be valid"),
        ));
        let query = RawQuery::new(
            QueryMetadata::new("findUser".to_owned(), None),
            "SELECT id FROM users WHERE email = /* @sqlcomp { type: param id: email valueType: string nullable: true } */ 'test@example.test' /* @sqlcomp { type: paramEnd } */;".to_owned(),
        )
        .with_analysis_sql("SELECT id FROM users WHERE email = ?;".to_owned())
        .with_param_usages(vec![crate::ParamUsage::new(
            "email".to_owned(),
            Some(crate::CoreType::String),
            true,
            location.clone(),
        )]);

        assert_eq!(
            query.analysis_sql(),
            "SELECT id FROM users WHERE email = ?;"
        );
        assert_eq!(query.param_usages().len(), 1);
        assert_eq!(query.param_usages()[0].id(), "email");
        assert_eq!(
            query.param_usages()[0].value_type_override(),
            Some(crate::CoreType::String)
        );
        assert!(query.param_usages()[0].nullable_override());
        assert_eq!(query.param_usages()[0].source_location(), &location);
    }

    #[test]
    fn raw_query_can_carry_slot_usages() {
        let location = SourceLocation::from_range(SourceRange::point(
            SourcePosition::one_based(8, 45).expect("test position should be valid"),
        ));
        let query = RawQuery::new(
            QueryMetadata::new("listUsers".to_owned(), None),
            "SELECT id FROM users WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly] } */;".to_owned(),
        )
        .with_analysis_sql("SELECT id FROM users WHERE 1 = 1;".to_owned())
        .with_slot_usages(vec![SlotUsage::new(
            "filter".to_owned(),
            vec!["activeOnly".to_owned()],
            32,
            location.clone(),
        )]);

        assert_eq!(query.analysis_sql(), "SELECT id FROM users WHERE 1 = 1;");
        assert_eq!(query.slot_usages().len(), 1);
        assert_eq!(query.slot_usages()[0].id(), "filter");
        assert_eq!(query.slot_usages()[0].targets(), ["activeOnly"]);
        assert_eq!(query.slot_usages()[0].insertion_index(), 32);
        assert_eq!(query.slot_usages()[0].source_location(), &location);
    }

    #[test]
    fn raw_fragment_preserves_metadata_sql_source_path_and_optional_source_location() {
        let location = SourceLocation::at_range(
            "sql/fragments.sql",
            SourceRange::point(
                SourcePosition::one_based(7, 1).expect("test position should be valid"),
            ),
        );
        let fragment = RawFragment::new(
            FragmentMetadata::new("activeOnly".to_owned()),
            "\nAND u.active = 1\n".to_owned(),
        )
        .with_source_path("sql/fragments.sql")
        .with_source_location(location.clone());

        assert_eq!(fragment.metadata().id(), "activeOnly");
        assert_eq!(fragment.sql(), "\nAND u.active = 1\n");
        assert_eq!(fragment.source_path(), Some(Path::new("sql/fragments.sql")));
        assert_eq!(fragment.source_location(), Some(&location));
    }

    #[test]
    fn raw_fragment_can_carry_analysis_sql_and_param_usages() {
        let location = SourceLocation::from_range(SourceRange::point(
            SourcePosition::one_based(8, 15).expect("test position should be valid"),
        ));
        let fragment = RawFragment::new(
            FragmentMetadata::new("byEmail".to_owned()),
            "\nAND u.email = /* @sqlcomp { type: param id: email valueType: string } */ 'ada@example.test' /* @sqlcomp { type: paramEnd } */\n".to_owned(),
        )
        .with_analysis_sql("\nAND u.email = ?\n".to_owned())
        .with_param_usages(vec![crate::ParamUsage::new(
            "email".to_owned(),
            Some(crate::CoreType::String),
            false,
            location.clone(),
        )]);

        assert_eq!(fragment.analysis_sql(), "\nAND u.email = ?\n");
        assert_eq!(fragment.param_usages().len(), 1);
        assert_eq!(fragment.param_usages()[0].id(), "email");
        assert_eq!(
            fragment.param_usages()[0].value_type_override(),
            Some(crate::CoreType::String)
        );
        assert_eq!(fragment.param_usages()[0].source_location(), &location);
    }

    #[test]
    fn analyzed_query_exposes_inferred_cardinality() {
        let analysis = AnalyzedQuery::new(Cardinality::Many);

        assert_eq!(analysis.cardinality(), Cardinality::Many);
    }
}
