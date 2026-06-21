use sqlay_core as core;

pub(super) struct FixtureColumnCoverage {
    pub(super) nullable_name: &'static str,
    pub(super) nullable_definition: &'static str,
    pub(super) not_null_name: &'static str,
    pub(super) not_null_definition: &'static str,
    pub(super) core_type: core::CoreType,
}

const fn fixture_column_coverage(
    nullable_name: &'static str,
    nullable_definition: &'static str,
    not_null_name: &'static str,
    not_null_definition: &'static str,
    core_type: core::CoreType,
) -> FixtureColumnCoverage {
    FixtureColumnCoverage {
        nullable_name,
        nullable_definition,
        not_null_name,
        not_null_definition,
        core_type,
    }
}

pub(super) static FIXTURE_ALL_COLUMN_TYPE_COVERAGE: &[FixtureColumnCoverage] = &[
    fixture_column_coverage(
        "tinyint_col",
        "tinyint_col TINYINT NULL",
        "tinyint_nn_col",
        "tinyint_nn_col TINYINT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "tinyint_unsigned_col",
        "tinyint_unsigned_col TINYINT UNSIGNED NULL",
        "tinyint_unsigned_nn_col",
        "tinyint_unsigned_nn_col TINYINT UNSIGNED NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "smallint_col",
        "smallint_col SMALLINT NULL",
        "smallint_nn_col",
        "smallint_nn_col SMALLINT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "smallint_unsigned_col",
        "smallint_unsigned_col SMALLINT UNSIGNED NULL",
        "smallint_unsigned_nn_col",
        "smallint_unsigned_nn_col SMALLINT UNSIGNED NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "mediumint_col",
        "mediumint_col MEDIUMINT NULL",
        "mediumint_nn_col",
        "mediumint_nn_col MEDIUMINT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "mediumint_unsigned_col",
        "mediumint_unsigned_col MEDIUMINT UNSIGNED NULL",
        "mediumint_unsigned_nn_col",
        "mediumint_unsigned_nn_col MEDIUMINT UNSIGNED NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "int_col",
        "int_col INT NULL",
        "int_nn_col",
        "int_nn_col INT NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "int_unsigned_col",
        "int_unsigned_col INT UNSIGNED NULL",
        "int_unsigned_nn_col",
        "int_unsigned_nn_col INT UNSIGNED NOT NULL",
        core::CoreType::Int64,
    ),
    fixture_column_coverage(
        "integer_col",
        "integer_col INTEGER NULL",
        "integer_nn_col",
        "integer_nn_col INTEGER NOT NULL",
        core::CoreType::Int32,
    ),
    fixture_column_coverage(
        "bigint_col",
        "bigint_col BIGINT NULL",
        "bigint_nn_col",
        "bigint_nn_col BIGINT NOT NULL PRIMARY KEY",
        core::CoreType::Int64,
    ),
    fixture_column_coverage(
        "bigint_unsigned_col",
        "bigint_unsigned_col BIGINT UNSIGNED NULL",
        "bigint_unsigned_nn_col",
        "bigint_unsigned_nn_col BIGINT UNSIGNED NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "decimal_18_4_col",
        "decimal_18_4_col DECIMAL(18, 4) NULL",
        "decimal_18_4_nn_col",
        "decimal_18_4_nn_col DECIMAL(18, 4) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "dec_col",
        "dec_col DEC(12, 2) NULL",
        "dec_nn_col",
        "dec_nn_col DEC(12, 2) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "numeric_col",
        "numeric_col NUMERIC(12, 2) NULL",
        "numeric_nn_col",
        "numeric_nn_col NUMERIC(12, 2) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "fixed_col",
        "fixed_col FIXED(12, 2) NULL",
        "fixed_nn_col",
        "fixed_nn_col FIXED(12, 2) NOT NULL",
        core::CoreType::Decimal,
    ),
    fixture_column_coverage(
        "float_col",
        "float_col FLOAT NULL",
        "float_nn_col",
        "float_nn_col FLOAT NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "double_col",
        "double_col DOUBLE NULL",
        "double_nn_col",
        "double_nn_col DOUBLE NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "double_precision_col",
        "double_precision_col DOUBLE PRECISION NULL",
        "double_precision_nn_col",
        "double_precision_nn_col DOUBLE PRECISION NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "real_col",
        "real_col REAL NULL",
        "real_nn_col",
        "real_nn_col REAL NOT NULL",
        core::CoreType::Float64,
    ),
    fixture_column_coverage(
        "bit_col",
        "bit_col BIT(8) NULL",
        "bit_nn_col",
        "bit_nn_col BIT(8) NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "bool_col",
        "bool_col BOOL NULL",
        "bool_nn_col",
        "bool_nn_col BOOL NOT NULL",
        core::CoreType::Bool,
    ),
    fixture_column_coverage(
        "boolean_col",
        "boolean_col BOOLEAN NULL",
        "boolean_nn_col",
        "boolean_nn_col BOOLEAN NOT NULL",
        core::CoreType::Bool,
    ),
    fixture_column_coverage(
        "date_col",
        "date_col DATE NULL",
        "date_nn_col",
        "date_nn_col DATE NOT NULL",
        core::CoreType::Date,
    ),
    fixture_column_coverage(
        "time_col",
        "time_col TIME NULL",
        "time_nn_col",
        "time_nn_col TIME NOT NULL",
        core::CoreType::Time,
    ),
    fixture_column_coverage(
        "datetime_6_col",
        "datetime_6_col DATETIME(6) NULL",
        "datetime_6_nn_col",
        "datetime_6_nn_col DATETIME(6) NOT NULL",
        core::CoreType::DateTime,
    ),
    fixture_column_coverage(
        "timestamp_col",
        "timestamp_col TIMESTAMP NULL DEFAULT NULL",
        "timestamp_nn_col",
        "timestamp_nn_col TIMESTAMP NOT NULL",
        core::CoreType::DateTime,
    ),
    fixture_column_coverage(
        "year_col",
        "year_col YEAR NULL",
        "year_nn_col",
        "year_nn_col YEAR NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "char_16_col",
        "char_16_col CHAR(16) NULL",
        "char_16_nn_col",
        "char_16_nn_col CHAR(16) NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "varchar_255_col",
        "varchar_255_col VARCHAR(255) NULL",
        "varchar_255_nn_col",
        "varchar_255_nn_col VARCHAR(255) NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "varchar_320_col",
        "varchar_320_col VARCHAR(320) NULL",
        "varchar_320_nn_col",
        "varchar_320_nn_col VARCHAR(320) NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "tinytext_col",
        "tinytext_col TINYTEXT NULL",
        "tinytext_nn_col",
        "tinytext_nn_col TINYTEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "text_col",
        "text_col TEXT NULL",
        "text_nn_col",
        "text_nn_col TEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "mediumtext_col",
        "mediumtext_col MEDIUMTEXT NULL",
        "mediumtext_nn_col",
        "mediumtext_nn_col MEDIUMTEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "longtext_col",
        "longtext_col LONGTEXT NULL",
        "longtext_nn_col",
        "longtext_nn_col LONGTEXT NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "enum_col",
        "enum_col ENUM('one', 'two') NULL",
        "enum_nn_col",
        "enum_nn_col ENUM('one', 'two') NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "set_col",
        "set_col SET('one', 'two') NULL",
        "set_nn_col",
        "set_nn_col SET('one', 'two') NOT NULL",
        core::CoreType::String,
    ),
    fixture_column_coverage(
        "binary_16_col",
        "binary_16_col BINARY(16) NULL",
        "binary_16_nn_col",
        "binary_16_nn_col BINARY(16) NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "varbinary_64_col",
        "varbinary_64_col VARBINARY(64) NULL",
        "varbinary_64_nn_col",
        "varbinary_64_nn_col VARBINARY(64) NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "tinyblob_col",
        "tinyblob_col TINYBLOB NULL",
        "tinyblob_nn_col",
        "tinyblob_nn_col TINYBLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "blob_col",
        "blob_col BLOB NULL",
        "blob_nn_col",
        "blob_nn_col BLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "mediumblob_col",
        "mediumblob_col MEDIUMBLOB NULL",
        "mediumblob_nn_col",
        "mediumblob_nn_col MEDIUMBLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "longblob_col",
        "longblob_col LONGBLOB NULL",
        "longblob_nn_col",
        "longblob_nn_col LONGBLOB NOT NULL",
        core::CoreType::Bytes,
    ),
    fixture_column_coverage(
        "json_col",
        "json_col JSON NULL",
        "json_nn_col",
        "json_nn_col JSON NOT NULL",
        core::CoreType::Json,
    ),
    fixture_column_coverage(
        "geometry_col",
        "geometry_col GEOMETRY NULL",
        "geometry_nn_col",
        "geometry_nn_col GEOMETRY NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "point_col",
        "point_col POINT NULL",
        "point_nn_col",
        "point_nn_col POINT NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "linestring_col",
        "linestring_col LINESTRING NULL",
        "linestring_nn_col",
        "linestring_nn_col LINESTRING NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "polygon_col",
        "polygon_col POLYGON NULL",
        "polygon_nn_col",
        "polygon_nn_col POLYGON NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "multipoint_col",
        "multipoint_col MULTIPOINT NULL",
        "multipoint_nn_col",
        "multipoint_nn_col MULTIPOINT NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "multilinestring_col",
        "multilinestring_col MULTILINESTRING NULL",
        "multilinestring_nn_col",
        "multilinestring_nn_col MULTILINESTRING NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "multipolygon_col",
        "multipolygon_col MULTIPOLYGON NULL",
        "multipolygon_nn_col",
        "multipolygon_nn_col MULTIPOLYGON NOT NULL",
        core::CoreType::Unknown,
    ),
    fixture_column_coverage(
        "geometrycollection_col",
        "geometrycollection_col GEOMETRYCOLLECTION NULL",
        "geometrycollection_nn_col",
        "geometrycollection_nn_col GEOMETRYCOLLECTION NOT NULL",
        core::CoreType::Unknown,
    ),
];

pub(super) fn assert_fixture_core_type_matrix(columns: &[core::DbResultColumn]) {
    for column in FIXTURE_ALL_COLUMN_TYPE_COVERAGE {
        assert_mapped_type(columns, column.nullable_name, column.core_type);
        assert_mapped_type(columns, column.not_null_name, column.core_type);
    }

    assert_mapped_type(columns, "childTimeCol", core::CoreType::Time);
}

pub(super) fn assert_fixture_nullability_matrix(columns: &[core::DbResultColumn]) {
    for column in FIXTURE_ALL_COLUMN_TYPE_COVERAGE {
        assert_mapped_nullability(columns, column.nullable_name, Some(true), true);
        assert_mapped_nullability(columns, column.not_null_name, Some(false), false);
    }
}

fn assert_mapped_type(columns: &[core::DbResultColumn], name: &str, expected_type: core::CoreType) {
    let column = columns
        .iter()
        .find(|column| column.name() == name)
        .unwrap_or_else(|| panic!("fixture should expose column `{name}`"));

    assert_eq!(column.ty(), expected_type, "{name} should map to core type");
}

fn assert_mapped_nullability(
    columns: &[core::DbResultColumn],
    name: &str,
    expected_metadata: Option<bool>,
    expected_output_nullable: bool,
) {
    let column = columns
        .iter()
        .find(|column| column.name() == name)
        .unwrap_or_else(|| panic!("fixture should expose column `{name}`"));

    assert_eq!(
        column.nullable(),
        expected_metadata,
        "{name} should preserve MySQL nullability metadata",
    );
    assert_eq!(
        column.to_result_column().is_nullable(),
        expected_output_nullable,
        "{name} should map to conservative Core IR output nullability",
    );
}

pub(super) fn fixture_all_column_type_columns(schema: &str) -> Vec<String> {
    let start_marker = "CREATE TABLE fixture_all_column_type (\n";
    let start = schema
        .find(start_marker)
        .expect("schema should define fixture_all_column_type")
        + start_marker.len();
    let end = schema[start..]
        .find("\n);")
        .expect("fixture_all_column_type definition should be closed")
        + start;

    schema[start..end]
        .lines()
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}
