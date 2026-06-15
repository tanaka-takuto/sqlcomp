use sqlcomp_core as core;

use crate::QueryCompiler;

/// Default application-owned query compiler.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultQueryCompiler;

impl QueryCompiler for DefaultQueryCompiler {
    fn compile(
        &self,
        query: &core::RawQuery,
        analysis: &core::AnalyzedQuery,
        metadata: &core::DbQueryMetadata,
    ) -> core::DiagnosticResult<core::CompiledQuery> {
        let cardinality = query
            .metadata()
            .cardinality()
            .unwrap_or_else(|| analysis.cardinality());
        let row = metadata
            .columns()
            .iter()
            .map(core::DbResultColumn::to_result_column)
            .collect();

        let mut compiled = core::CompiledQuery::new(
            core::QueryId::new(query.metadata().id().to_owned()),
            query.sql().to_owned(),
            cardinality,
            Vec::new(),
            row,
        );

        if let Some(source_path) = query.source_path() {
            compiled = compiled.with_source_path(source_path.to_path_buf());
        }

        Ok(compiled)
    }
}
