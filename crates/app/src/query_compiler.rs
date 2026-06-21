use sqlay_core as core;

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
        let (input, params) = compile_param_bindings(query, metadata)?;
        let row = metadata
            .columns()
            .iter()
            .map(core::DbResultColumn::to_result_column)
            .collect();

        let mut compiled = core::CompiledQuery::new(
            core::QueryId::new(query.metadata().id().to_owned()),
            query.analysis_sql().to_owned(),
            cardinality,
            input,
            row,
        )
        .with_params(params);

        if let Some(source_path) = query.source_path() {
            compiled = compiled.with_source_path(source_path.to_path_buf());
        }

        Ok(compiled)
    }
}

fn compile_param_bindings(
    query: &core::RawQuery,
    metadata: &core::DbQueryMetadata,
) -> core::DiagnosticResult<(Vec<core::InputField>, Vec<core::ParamBinding>)> {
    if query.param_usages().len() != metadata.param_usages().len() {
        return Err(query_error(
            query,
            format!(
                "resolved Param usage count {} does not match source Param usage count {}",
                metadata.param_usages().len(),
                query.param_usages().len()
            ),
        ));
    }

    let mut input = Vec::<core::InputField>::new();
    let mut params = Vec::with_capacity(query.param_usages().len());

    for (source_usage, resolved_usage) in query.param_usages().iter().zip(metadata.param_usages()) {
        if source_usage.id() != resolved_usage.id() {
            return Err(param_usage_error(
                query,
                source_usage,
                format!(
                    "resolved Param metadata id `{}` does not match source Param id `{}`",
                    resolved_usage.id(),
                    source_usage.id()
                ),
            ));
        }

        let nullable = source_usage.nullable_override();
        if let Some(existing) = input.iter().find(|field| field.name() == source_usage.id()) {
            if existing.ty() != resolved_usage.ty() {
                return Err(param_usage_error(
                    query,
                    source_usage,
                    format!(
                        "conflicting Param `{}` types: first occurrence resolved to {:?} but later occurrence resolved to {:?}",
                        source_usage.id(),
                        existing.ty(),
                        resolved_usage.ty()
                    ),
                ));
            }
            if existing.is_nullable() != nullable {
                return Err(param_usage_error(
                    query,
                    source_usage,
                    format!(
                        "conflicting Param `{}` nullability: first occurrence is nullable {} but later occurrence is nullable {}",
                        source_usage.id(),
                        existing.is_nullable(),
                        nullable
                    ),
                ));
            }
        } else {
            input.push(core::InputField::new(
                source_usage.id().to_owned(),
                resolved_usage.ty(),
                nullable,
            ));
        }

        params.push(core::ParamBinding::new(
            source_usage.id().to_owned(),
            resolved_usage.ty(),
            nullable,
        ));
    }

    Ok((input, params))
}

fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

fn param_usage_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            query
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    core::DiagnosticReport::new(core::Diagnostic::error(message).with_location(location))
}
