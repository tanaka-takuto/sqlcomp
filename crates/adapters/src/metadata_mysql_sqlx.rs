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
    ) -> core::DbQueryMetadata {
        core::DbQueryMetadata
    }
}
