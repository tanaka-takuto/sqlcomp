use crate::{CoreType, CoreTypeRef, ResultColumn};

/// Database metadata description normalized for compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbQueryMetadata {
    columns: Vec<DbResultColumn>,
    param_usages: Vec<DbParamUsage>,
}

impl DbQueryMetadata {
    /// Build database query metadata.
    #[must_use]
    pub const fn new(columns: Vec<DbResultColumn>) -> Self {
        Self {
            columns,
            param_usages: Vec::new(),
        }
    }

    /// Attach resolved Param usage metadata in source occurrence order.
    #[must_use]
    pub fn with_param_usages(mut self, param_usages: Vec<DbParamUsage>) -> Self {
        self.param_usages = param_usages;
        self
    }

    /// Result columns described by the database metadata provider.
    #[must_use]
    pub fn columns(&self) -> &[DbResultColumn] {
        &self.columns
    }

    /// Resolved Param usage metadata in source occurrence order.
    #[must_use]
    pub fn param_usages(&self) -> &[DbParamUsage] {
        &self.param_usages
    }
}

/// Database metadata description normalized for mutation compilation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DbMutationMetadata {
    param_usages: Vec<DbParamUsage>,
}

impl DbMutationMetadata {
    /// Build empty database mutation metadata.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            param_usages: Vec::new(),
        }
    }

    /// Attach resolved Param usage metadata in source occurrence order.
    #[must_use]
    pub fn with_param_usages(mut self, param_usages: Vec<DbParamUsage>) -> Self {
        self.param_usages = param_usages;
        self
    }

    /// Resolved Param usage metadata in source occurrence order.
    #[must_use]
    pub fn param_usages(&self) -> &[DbParamUsage] {
        &self.param_usages
    }
}

/// Result column metadata from a database provider before final IR emission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbResultColumn {
    name: String,
    type_ref: CoreTypeRef,
    nullable: Option<bool>,
}

impl DbResultColumn {
    /// Build a database result column metadata value.
    #[must_use]
    pub const fn new(name: String, ty: CoreType, nullable: Option<bool>) -> Self {
        Self::new_type_ref(name, CoreTypeRef::Scalar(ty), nullable)
    }

    /// Build a database result column metadata value from a richer Core type reference.
    #[must_use]
    pub const fn new_type_ref(name: String, type_ref: CoreTypeRef, nullable: Option<bool>) -> Self {
        Self {
            name,
            type_ref,
            nullable,
        }
    }

    /// Column name exactly as reported by the database metadata provider.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Language-neutral column type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.type_ref.core_type()
    }

    /// Language-neutral column type reference.
    #[must_use]
    pub const fn type_ref(&self) -> &CoreTypeRef {
        &self.type_ref
    }

    /// Database nullability metadata, when the provider can determine it.
    #[must_use]
    pub const fn nullable(&self) -> Option<bool> {
        self.nullable
    }

    /// Conservative nullability for generated output.
    #[must_use]
    pub fn is_nullable_for_output(&self) -> bool {
        self.nullable.unwrap_or(true)
    }

    /// Convert database metadata into a compiled result column.
    #[must_use]
    pub fn to_result_column(&self) -> ResultColumn {
        ResultColumn::new_type_ref(
            self.name.clone(),
            self.type_ref.clone(),
            self.is_nullable_for_output(),
        )
    }
}

/// Database-backed type metadata for one Param occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbParamUsage {
    id: String,
    type_ref: CoreTypeRef,
}

impl DbParamUsage {
    /// Build resolved Param usage metadata.
    #[must_use]
    pub const fn new(id: String, ty: CoreType) -> Self {
        Self::new_type_ref(id, CoreTypeRef::Scalar(ty))
    }

    /// Build resolved Param usage metadata from a richer Core type reference.
    #[must_use]
    pub const fn new_type_ref(id: String, type_ref: CoreTypeRef) -> Self {
        Self { id, type_ref }
    }

    /// Param ID exactly as written in source metadata.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Language-neutral Param type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.type_ref.core_type()
    }

    /// Language-neutral Param type reference.
    #[must_use]
    pub const fn type_ref(&self) -> &CoreTypeRef {
        &self.type_ref
    }
}

#[cfg(test)]
mod tests {
    use crate::{CoreType, DbMutationMetadata, DbParamUsage, DbQueryMetadata, DbResultColumn};

    #[test]
    fn db_query_metadata_preserves_result_column_metadata() {
        let columns = vec![
            DbResultColumn::new("userId".to_owned(), CoreType::Int64, Some(false)),
            DbResultColumn::new("nickname".to_owned(), CoreType::String, Some(true)),
        ];
        let metadata = DbQueryMetadata::new(columns.clone());

        assert_eq!(metadata.columns(), columns);
        assert!(metadata.param_usages().is_empty());
    }

    #[test]
    fn db_query_metadata_preserves_resolved_param_usage_metadata() {
        let metadata = DbQueryMetadata::new(Vec::new()).with_param_usages(vec![
            DbParamUsage::new("email".to_owned(), CoreType::String),
            DbParamUsage::new("userId".to_owned(), CoreType::Int64),
        ]);

        assert_eq!(
            metadata.param_usages(),
            [
                DbParamUsage::new("email".to_owned(), CoreType::String),
                DbParamUsage::new("userId".to_owned(), CoreType::Int64),
            ]
        );
    }

    #[test]
    fn database_metadata_conservatively_treats_unknown_nullability_as_nullable() {
        let metadata = DbQueryMetadata::new(vec![DbResultColumn::new(
            "mystery".to_owned(),
            CoreType::Unknown,
            None,
        )]);
        let column = metadata.columns()[0].to_result_column();

        assert_eq!(column.name(), "mystery");
        assert_eq!(column.ty(), CoreType::Unknown);
        assert!(column.is_nullable());
    }

    #[test]
    fn db_mutation_metadata_preserves_resolved_param_usage_metadata_without_columns() {
        let metadata = DbMutationMetadata::new().with_param_usages(vec![
            DbParamUsage::new("email".to_owned(), CoreType::String),
            DbParamUsage::new("userId".to_owned(), CoreType::Int64),
        ]);

        assert_eq!(
            metadata.param_usages(),
            [
                DbParamUsage::new("email".to_owned(), CoreType::String),
                DbParamUsage::new("userId".to_owned(), CoreType::Int64),
            ]
        );
    }
}
