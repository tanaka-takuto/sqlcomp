//! sqlx-backed `MySQL` metadata adapter.

use sqlcomp_app::MetadataProvider;
use sqlcomp_core as core;

/// Dummy sqlx-backed `MySQL` metadata provider.
#[derive(Clone, Copy, Debug, Default)]
pub struct SqlxMysqlMetadataProvider;

impl MetadataProvider for SqlxMysqlMetadataProvider {
    fn describe(
        &self,
        _query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        Ok(core::DbQueryMetadata::new(Vec::new()))
    }
}

/// Map one `MySQL` result column description into core metadata.
#[must_use]
pub fn map_mysql_result_column_metadata(
    name: &str,
    type_name: &str,
    nullable: Option<bool>,
) -> core::DbResultColumn {
    core::DbResultColumn::new(
        name.to_owned(),
        mysql_type_name_to_core_type(type_name),
        nullable,
    )
}

fn mysql_type_name_to_core_type(type_name: &str) -> core::CoreType {
    let normalized = normalized_mysql_type_name(type_name);
    let (base_type, is_unsigned) = normalized
        .strip_suffix(" UNSIGNED")
        .map_or((normalized.as_str(), false), |base_type| (base_type, true));

    match base_type {
        "BOOL" | "BOOLEAN" => core::CoreType::Bool,
        "INT" | "INTEGER" if is_unsigned => core::CoreType::Int64,
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" => core::CoreType::Int32,
        "BIGINT" if is_unsigned => core::CoreType::Unknown,
        "BIGINT" => core::CoreType::Int64,
        "DEC" | "DECIMAL" | "FIXED" | "NUMERIC" => core::CoreType::Decimal,
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" | "REAL" => core::CoreType::Float64,
        "CHAR" | "ENUM" | "LONGTEXT" | "MEDIUMTEXT" | "SET" | "TEXT" | "TINYTEXT" | "VARCHAR" => {
            core::CoreType::String
        }
        "BINARY" | "BLOB" | "LONGBLOB" | "MEDIUMBLOB" | "TINYBLOB" | "VARBINARY" => {
            core::CoreType::Bytes
        }
        "DATE" => core::CoreType::Date,
        "TIME" => core::CoreType::Time,
        "DATETIME" | "TIMESTAMP" => core::CoreType::DateTime,
        "JSON" => core::CoreType::Json,
        _ => core::CoreType::Unknown,
    }
}

fn normalized_mysql_type_name(type_name: &str) -> String {
    let mut without_precision = String::with_capacity(type_name.len());
    let mut precision_depth = 0_u8;

    for character in type_name.trim().chars() {
        match character {
            '(' => precision_depth = precision_depth.saturating_add(1),
            ')' if precision_depth > 0 => precision_depth -= 1,
            _ if precision_depth == 0 => without_precision.push(character),
            _ => {}
        }
    }

    let mut collapsed = String::with_capacity(without_precision.len());
    for word in without_precision.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }

    collapsed.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::map_mysql_result_column_metadata;
    use sqlcomp_core as core;

    #[test]
    fn maps_representative_mysql_type_names_to_core_types() {
        let cases = [
            ("BOOLEAN", core::CoreType::Bool),
            ("TINYINT", core::CoreType::Int32),
            ("SMALLINT", core::CoreType::Int32),
            ("MEDIUMINT", core::CoreType::Int32),
            ("INT", core::CoreType::Int32),
            ("INTEGER", core::CoreType::Int32),
            ("BIGINT", core::CoreType::Int64),
            ("DECIMAL", core::CoreType::Decimal),
            ("NUMERIC", core::CoreType::Decimal),
            ("FLOAT", core::CoreType::Float64),
            ("DOUBLE", core::CoreType::Float64),
            ("REAL", core::CoreType::Float64),
            ("CHAR", core::CoreType::String),
            ("VARCHAR", core::CoreType::String),
            ("TEXT", core::CoreType::String),
            ("TINYTEXT", core::CoreType::String),
            ("MEDIUMTEXT", core::CoreType::String),
            ("LONGTEXT", core::CoreType::String),
            ("ENUM", core::CoreType::String),
            ("SET", core::CoreType::String),
            ("BINARY", core::CoreType::Bytes),
            ("VARBINARY", core::CoreType::Bytes),
            ("BLOB", core::CoreType::Bytes),
            ("TINYBLOB", core::CoreType::Bytes),
            ("MEDIUMBLOB", core::CoreType::Bytes),
            ("LONGBLOB", core::CoreType::Bytes),
            ("DATE", core::CoreType::Date),
            ("TIME", core::CoreType::Time),
            ("DATETIME", core::CoreType::DateTime),
            ("TIMESTAMP", core::CoreType::DateTime),
            ("JSON", core::CoreType::Json),
        ];

        for (type_name, expected_type) in cases {
            let column = map_mysql_result_column_metadata("value", type_name, Some(false));

            assert_eq!(
                column,
                core::DbResultColumn::new("value".to_owned(), expected_type, Some(false)),
                "{type_name} should map to {expected_type:?}"
            );
        }
    }

    #[test]
    fn maps_unknown_mysql_type_names_conservatively() {
        let column = map_mysql_result_column_metadata("shape", "GEOMETRY", Some(false));

        assert_eq!(
            column,
            core::DbResultColumn::new("shape".to_owned(), core::CoreType::Unknown, Some(false))
        );
    }

    #[test]
    fn preserves_unknown_nullability_for_core_ir() {
        let column = map_mysql_result_column_metadata("name", "VARCHAR", None);

        assert_eq!(
            column,
            core::DbResultColumn::new("name".to_owned(), core::CoreType::String, None)
        );
        assert!(column.to_result_column().is_nullable());
    }

    #[test]
    fn normalizes_case_and_precision_suffixes() {
        let column = map_mysql_result_column_metadata("amount", "decimal(18, 4)", Some(false));

        assert_eq!(
            column,
            core::DbResultColumn::new("amount".to_owned(), core::CoreType::Decimal, Some(false))
        );

        let widened = map_mysql_result_column_metadata("count", "int(10) unsigned", Some(false));

        assert_eq!(
            widened,
            core::DbResultColumn::new("count".to_owned(), core::CoreType::Int64, Some(false))
        );

        let unknown = map_mysql_result_column_metadata("id", "BIGINT UNSIGNED", Some(false));

        assert_eq!(
            unknown,
            core::DbResultColumn::new("id".to_owned(), core::CoreType::Unknown, Some(false))
        );
    }
}
