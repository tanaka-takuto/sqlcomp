use std::path::{Path, PathBuf};

use crate::SourceLocation;

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

/// Metadata parsed from an MVP `type: query` annotation.
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

/// Raw query extracted from SQL source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawQuery {
    metadata: QueryMetadata,
    sql: String,
    source_path: Option<PathBuf>,
    source_location: Option<SourceLocation>,
}

impl RawQuery {
    /// Build a raw query from parsed metadata and SQL text.
    #[must_use]
    pub const fn new(metadata: QueryMetadata, sql: String) -> Self {
        Self {
            metadata,
            sql,
            source_path: None,
            source_location: None,
        }
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
        AnalyzedQuery, Cardinality, QueryMetadata, RawQuery, SourceLocation, SourcePosition,
        SourceRange,
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
    fn analyzed_query_exposes_inferred_cardinality() {
        let analysis = AnalyzedQuery::new(Cardinality::Many);

        assert_eq!(analysis.cardinality(), Cardinality::Many);
    }
}
