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
        if !query.param_usages().is_empty() {
            return Err(query_error(
                query,
                "Param queries are not supported by generated output yet",
            ));
        }

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
            query.analysis_sql().to_owned(),
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

fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}
