use std::path::{Path, PathBuf};

use crate::{Cardinality, QueryId};

/// Language-neutral compiled query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledQuery {
    id: QueryId,
    sql: String,
    cardinality: Cardinality,
    source_path: Option<PathBuf>,
    input: Vec<InputField>,
    params: Vec<ParamBinding>,
    row: Vec<ResultColumn>,
}

impl CompiledQuery {
    /// Build a compiled query Core IR value.
    #[must_use]
    pub const fn new(
        id: QueryId,
        sql: String,
        cardinality: Cardinality,
        input: Vec<InputField>,
        row: Vec<ResultColumn>,
    ) -> Self {
        Self {
            id,
            sql,
            cardinality,
            source_path: None,
            input,
            params: Vec::new(),
            row,
        }
    }

    /// Attach query parameter bindings in source occurrence order.
    #[must_use]
    pub fn with_params(mut self, params: Vec<ParamBinding>) -> Self {
        self.params = params;
        self
    }

    /// Attach the source SQL path relative to the configuration directory.
    #[must_use]
    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Query ID exactly as written in source metadata.
    #[must_use]
    pub const fn id(&self) -> &QueryId {
        &self.id
    }

    /// SQL text for the compiled query.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Query result cardinality independent from any target-language syntax.
    #[must_use]
    pub const fn cardinality(&self) -> Cardinality {
        self.cardinality
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Input fields for the query. MVP queries have an empty input list.
    #[must_use]
    pub fn input(&self) -> &[InputField] {
        &self.input
    }

    /// Query parameter bindings in source occurrence order.
    #[must_use]
    pub fn params(&self) -> &[ParamBinding] {
        &self.params
    }

    /// Result row columns for the query.
    #[must_use]
    pub fn row(&self) -> &[ResultColumn] {
        &self.row
    }
}

/// Query input field in Core IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputField {
    name: String,
    ty: CoreType,
    nullable: bool,
}

impl InputField {
    /// Build a query input field.
    #[must_use]
    pub const fn new(name: String, ty: CoreType, nullable: bool) -> Self {
        Self { name, ty, nullable }
    }

    /// Input field name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Language-neutral input type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.ty
    }

    /// Whether the input field accepts null.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        self.nullable
    }
}

/// One generated parameter binding in source occurrence order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParamBinding {
    input_name: String,
    ty: CoreType,
    nullable: bool,
}

impl ParamBinding {
    /// Build a query parameter binding.
    #[must_use]
    pub const fn new(input_name: String, ty: CoreType, nullable: bool) -> Self {
        Self {
            input_name,
            ty,
            nullable,
        }
    }

    /// Input field name used for this parameter occurrence.
    #[must_use]
    pub fn input_name(&self) -> &str {
        &self.input_name
    }

    /// Language-neutral parameter type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.ty
    }

    /// Whether this parameter occurrence accepts null.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        self.nullable
    }
}

/// Result row column in Core IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultColumn {
    name: String,
    ty: CoreType,
    nullable: bool,
}

impl ResultColumn {
    /// Build a result row column.
    #[must_use]
    pub const fn new(name: String, ty: CoreType, nullable: bool) -> Self {
        Self { name, ty, nullable }
    }

    /// Result column name exactly as reported by database metadata.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Language-neutral result column type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.ty
    }

    /// Whether generated output should treat this column as nullable.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        self.nullable
    }
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
    /// Time value.
    Time,
    /// Date-time value.
    DateTime,
    /// JSON value.
    Json,
    /// Unknown database type.
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{Cardinality, CompiledQuery, CoreType, QueryId, ResultColumn};

    #[test]
    fn compiled_query_represents_empty_mvp_input_and_result_columns() {
        let query = CompiledQuery::new(
            QueryId::new("listUsers".to_owned()),
            "SELECT id, name FROM users;".to_owned(),
            Cardinality::Many,
            Vec::new(),
            vec![
                ResultColumn::new("id".to_owned(), CoreType::Int64, false),
                ResultColumn::new("name".to_owned(), CoreType::String, true),
            ],
        );

        assert_eq!(query.id().as_str(), "listUsers");
        assert_eq!(query.sql(), "SELECT id, name FROM users;");
        assert_eq!(query.cardinality(), Cardinality::Many);
        assert_eq!(query.source_path(), None);
        assert!(query.input().is_empty());
        assert!(query.params().is_empty());
        assert_eq!(query.row().len(), 2);
        assert_eq!(query.row()[0].name(), "id");
        assert_eq!(query.row()[0].ty(), CoreType::Int64);
        assert!(!query.row()[0].is_nullable());
        assert_eq!(query.row()[1].name(), "name");
        assert_eq!(query.row()[1].ty(), CoreType::String);
        assert!(query.row()[1].is_nullable());
    }

    #[test]
    fn compiled_query_preserves_source_path_when_available() {
        let query = CompiledQuery::new(
            QueryId::new("listUsers".to_owned()),
            "SELECT id FROM users;".to_owned(),
            Cardinality::Many,
            Vec::new(),
            Vec::new(),
        )
        .with_source_path("sql/users.sql");

        assert_eq!(query.source_path(), Some(Path::new("sql/users.sql")));
    }
}
