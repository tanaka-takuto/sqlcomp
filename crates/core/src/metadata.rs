use crate::{CoreType, ResultColumn};

/// Database metadata description normalized for compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbQueryMetadata {
    columns: Vec<DbResultColumn>,
}

impl DbQueryMetadata {
    /// Build database query metadata.
    #[must_use]
    pub const fn new(columns: Vec<DbResultColumn>) -> Self {
        Self { columns }
    }

    /// Result columns described by the database metadata provider.
    #[must_use]
    pub fn columns(&self) -> &[DbResultColumn] {
        &self.columns
    }
}

/// Result column metadata from a database provider before final IR emission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DbResultColumn {
    name: String,
    ty: CoreType,
    nullable: Option<bool>,
}

impl DbResultColumn {
    /// Build a database result column metadata value.
    #[must_use]
    pub const fn new(name: String, ty: CoreType, nullable: Option<bool>) -> Self {
        Self { name, ty, nullable }
    }

    /// Column name exactly as reported by the database metadata provider.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Language-neutral column type.
    #[must_use]
    pub const fn ty(&self) -> CoreType {
        self.ty
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
        ResultColumn::new(self.name.clone(), self.ty, self.is_nullable_for_output())
    }
}

#[cfg(test)]
mod tests {
    use crate::{CoreType, DbQueryMetadata, DbResultColumn};

    #[test]
    fn db_query_metadata_preserves_result_column_metadata() {
        let columns = vec![
            DbResultColumn::new("userId".to_owned(), CoreType::Int64, Some(false)),
            DbResultColumn::new("nickname".to_owned(), CoreType::String, Some(true)),
        ];
        let metadata = DbQueryMetadata::new(columns.clone());

        assert_eq!(metadata.columns(), columns);
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
}
