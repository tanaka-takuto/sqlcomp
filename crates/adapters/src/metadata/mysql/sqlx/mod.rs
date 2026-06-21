//! sqlx-backed `MySQL` metadata adapter.

mod describe;
mod diagnostics;
mod param_inference;
mod result_mapping;
mod schema_columns;

pub use describe::SqlxMysqlMetadataProvider;
pub use result_mapping::map_mysql_result_column_metadata;
