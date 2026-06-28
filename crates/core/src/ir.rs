use std::path::{Path, PathBuf};

use crate::{Cardinality, MutationId, QueryId};

mod dynamic;

pub use dynamic::{
    CompiledDynamicQuery, CompiledRepeatDefinition, CompiledRepeatOccurrence, CompiledSlotBranch,
    CompiledSlotDefinition, CompiledSlotOccurrence, CompiledSqlBody, CompiledSqlSegment,
};

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

/// Language-neutral compiled mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledMutation {
    id: MutationId,
    sql: String,
    kind: MutationKind,
    source_path: Option<PathBuf>,
    input: Vec<InputField>,
    params: Vec<ParamBinding>,
    dynamic_body: Option<CompiledDynamicQuery>,
}

impl CompiledMutation {
    /// Build a compiled mutation Core IR value.
    #[must_use]
    pub const fn new(
        id: MutationId,
        sql: String,
        kind: MutationKind,
        input: Vec<InputField>,
    ) -> Self {
        Self {
            id,
            sql,
            kind,
            source_path: None,
            input,
            params: Vec::new(),
            dynamic_body: None,
        }
    }

    /// Attach mutation parameter bindings in source occurrence order.
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

    /// Attach dynamic SQL body data for a mutation with Slot occurrences.
    #[must_use]
    pub fn with_dynamic_body(mut self, body: CompiledDynamicQuery) -> Self {
        self.dynamic_body = Some(body);
        self
    }

    /// Mutation ID exactly as written in source metadata.
    #[must_use]
    pub const fn id(&self) -> &MutationId {
        &self.id
    }

    /// SQL text for the compiled mutation.
    #[must_use]
    pub fn sql(&self) -> &str {
        &self.sql
    }

    /// Supported mutation statement family.
    #[must_use]
    pub const fn kind(&self) -> MutationKind {
        self.kind
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Input fields for the mutation. Mutations without params have an empty input list.
    #[must_use]
    pub fn input(&self) -> &[InputField] {
        &self.input
    }

    /// Mutation parameter bindings in source occurrence order.
    #[must_use]
    pub fn params(&self) -> &[ParamBinding] {
        &self.params
    }

    /// Dynamic Slot body for this mutation, when the mutation contains Slots.
    #[must_use]
    pub const fn dynamic_body(&self) -> Option<&CompiledDynamicQuery> {
        self.dynamic_body.as_ref()
    }
}

/// Supported mutation statement family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationKind {
    /// `MySQL` `INSERT`.
    Insert,
    /// `MySQL` `UPDATE`.
    Update,
    /// `MySQL` `DELETE`.
    Delete,
    /// `MySQL` `REPLACE`.
    Replace,
}

/// Language-neutral compiled builder in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledBuilder {
    /// A compiled SELECT query builder.
    Query(CompiledQuery),
    /// A compiled mutation builder.
    Mutation(CompiledMutation),
}

impl CompiledBuilder {
    /// Builder ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Query(query) => query.id().as_str(),
            Self::Mutation(mutation) => mutation.id().as_str(),
        }
    }

    /// Source SQL path relative to the configuration directory, when known.
    #[must_use]
    pub fn source_path(&self) -> Option<&Path> {
        match self {
            Self::Query(query) => query.source_path(),
            Self::Mutation(mutation) => mutation.source_path(),
        }
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
