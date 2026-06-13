//! `MySQL` dialect analysis adapter.

use sqlcomp_app::DialectAnalyzer;
use sqlcomp_core as core;

/// Dummy `MySQL` dialect analyzer.
#[derive(Clone, Copy, Debug, Default)]
pub struct MysqlDialectAnalyzer;

impl DialectAnalyzer for MysqlDialectAnalyzer {
    fn analyze(&self, _query: &core::RawQuery) -> core::AnalyzedQuery {
        core::AnalyzedQuery
    }
}
