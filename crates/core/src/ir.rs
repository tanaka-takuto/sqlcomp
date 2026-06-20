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
    dynamic_body: Option<CompiledDynamicQuery>,
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
            dynamic_body: None,
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

    /// Attach dynamic SQL body data for a query with Slot occurrences.
    #[must_use]
    pub fn with_dynamic_body(mut self, body: CompiledDynamicQuery) -> Self {
        self.dynamic_body = Some(body);
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

    /// Input fields for the query. Queries without params have an empty input list.
    #[must_use]
    pub fn input(&self) -> &[InputField] {
        &self.input
    }

    /// Query parameter bindings in source occurrence order.
    #[must_use]
    pub fn params(&self) -> &[ParamBinding] {
        &self.params
    }

    /// Dynamic Slot body for this query, when the query contains Slots.
    #[must_use]
    pub const fn dynamic_body(&self) -> Option<&CompiledDynamicQuery> {
        self.dynamic_body.as_ref()
    }

    /// Result row columns for the query.
    #[must_use]
    pub fn row(&self) -> &[ResultColumn] {
        &self.row
    }
}

/// Runtime-composable SQL body for a query with Slot occurrences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledDynamicQuery {
    base_segments: Vec<CompiledSqlSegment>,
    slot_occurrences: Vec<CompiledSlotOccurrence>,
    slots: Vec<CompiledSlotDefinition>,
}

impl CompiledDynamicQuery {
    /// Build a dynamic query body.
    ///
    /// `base_segments` contains the SQL text around Slot occurrences, so callers
    /// should provide exactly one more base segment than Slot occurrence.
    #[must_use]
    pub const fn new(
        base_segments: Vec<CompiledSqlSegment>,
        slot_occurrences: Vec<CompiledSlotOccurrence>,
        slots: Vec<CompiledSlotDefinition>,
    ) -> Self {
        Self {
            base_segments,
            slot_occurrences,
            slots,
        }
    }

    /// Base SQL segments around Slot occurrences.
    #[must_use]
    pub fn base_segments(&self) -> &[CompiledSqlSegment] {
        &self.base_segments
    }

    /// Slot occurrences in query SQL order.
    #[must_use]
    pub fn slot_occurrences(&self) -> &[CompiledSlotOccurrence] {
        &self.slot_occurrences
    }

    /// Unique Slot definitions in query first-seen order.
    #[must_use]
    pub fn slots(&self) -> &[CompiledSlotDefinition] {
        &self.slots
    }
}

/// One SQL segment and the Param bindings it contains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSqlSegment {
    sql: String,
    params: Vec<ParamBinding>,
}

impl CompiledSqlSegment {
    /// Build a compiled SQL segment.
    #[must_use]
    pub const fn new(sql: String, params: Vec<ParamBinding>) -> Self {
        Self { sql, params }
    }

    /// SQL text for this segment.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Param bindings in this segment in SQL placeholder order.
    #[must_use]
    pub fn params(&self) -> &[ParamBinding] {
        &self.params
    }
}

/// One occurrence of a query-local Slot in SQL order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSlotOccurrence {
    slot_id: String,
}

impl CompiledSlotOccurrence {
    /// Build a compiled Slot occurrence.
    #[must_use]
    pub const fn new(slot_id: String) -> Self {
        Self { slot_id }
    }

    /// Query-local Slot ID for this occurrence.
    #[must_use]
    pub fn slot_id(&self) -> &str {
        &self.slot_id
    }
}

/// Unique Slot definition and its ordered target branches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSlotDefinition {
    id: String,
    branches: Vec<CompiledSlotBranch>,
}

impl CompiledSlotDefinition {
    /// Build a compiled Slot definition.
    #[must_use]
    pub const fn new(id: String, branches: Vec<CompiledSlotBranch>) -> Self {
        Self { id, branches }
    }

    /// Query-local Slot ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Target branches in source `targets` order.
    #[must_use]
    pub fn branches(&self) -> &[CompiledSlotBranch] {
        &self.branches
    }
}

/// One selected Fragment branch for a Slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSlotBranch {
    target_id: String,
    segments: Vec<CompiledSqlSegment>,
}

impl CompiledSlotBranch {
    /// Build a compiled Slot branch.
    #[must_use]
    pub const fn new(target_id: String, segments: Vec<CompiledSqlSegment>) -> Self {
        Self {
            target_id,
            segments,
        }
    }

    /// Fragment ID selected by this branch.
    #[must_use]
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Fragment SQL segments for this branch.
    #[must_use]
    pub fn segments(&self) -> &[CompiledSqlSegment] {
        &self.segments
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

    use crate::{
        Cardinality, CompiledDynamicQuery, CompiledQuery, CompiledSlotBranch,
        CompiledSlotDefinition, CompiledSlotOccurrence, CompiledSqlSegment, CoreType, ParamBinding,
        QueryId, ResultColumn,
    };

    #[test]
    fn compiled_query_represents_empty_paramless_input_and_result_columns() {
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

    #[test]
    fn compiled_query_can_carry_dynamic_slot_body() {
        let dynamic_body = CompiledDynamicQuery::new(
            vec![
                CompiledSqlSegment::new(
                    "SELECT id FROM users WHERE active = ?".to_owned(),
                    vec![ParamBinding::new(
                        "active".to_owned(),
                        CoreType::Bool,
                        false,
                    )],
                ),
                CompiledSqlSegment::new(";".to_owned(), Vec::new()),
            ],
            vec![CompiledSlotOccurrence::new("filter".to_owned())],
            vec![CompiledSlotDefinition::new(
                "filter".to_owned(),
                vec![CompiledSlotBranch::new(
                    "byEmail".to_owned(),
                    vec![CompiledSqlSegment::new(
                        " AND email = ?".to_owned(),
                        vec![ParamBinding::new(
                            "email".to_owned(),
                            CoreType::String,
                            false,
                        )],
                    )],
                )],
            )],
        );
        let query = CompiledQuery::new(
            QueryId::new("listUsers".to_owned()),
            "SELECT id FROM users WHERE active = ?;".to_owned(),
            Cardinality::Many,
            Vec::new(),
            Vec::new(),
        )
        .with_dynamic_body(dynamic_body);

        let dynamic_body = query
            .dynamic_body()
            .expect("dynamic body should be present");

        assert_eq!(dynamic_body.base_segments().len(), 2);
        assert_eq!(dynamic_body.slot_occurrences()[0].slot_id(), "filter");
        assert_eq!(dynamic_body.slots()[0].branches()[0].target_id(), "byEmail");
    }
}
