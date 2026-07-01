use sqlay_core as core;

use crate::{MutationCompiler, QueryCompiler};

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
        let (input, params) = compile_param_bindings(
            query.param_usages(),
            metadata.param_usages(),
            query.source_location(),
        )?;
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

impl MutationCompiler for DefaultQueryCompiler {
    fn compile_mutation(
        &self,
        mutation: &core::RawMutation,
        analysis: &core::AnalyzedMutation,
        metadata: &core::DbMutationMetadata,
    ) -> core::DiagnosticResult<core::CompiledMutation> {
        let (input, params) = compile_param_bindings(
            mutation.param_usages(),
            metadata.param_usages(),
            mutation.source_location(),
        )?;

        let mut compiled = core::CompiledMutation::new(
            core::MutationId::new(mutation.metadata().id().to_owned()),
            mutation.analysis_sql().to_owned(),
            analysis.kind(),
            input,
        )
        .with_params(params);

        if let Some(source_path) = mutation.source_path() {
            compiled = compiled.with_source_path(source_path.to_path_buf());
        }

        Ok(compiled)
    }
}

fn compile_param_bindings(
    source_param_usages: &[core::ParamUsage],
    resolved_param_usages: &[core::DbParamUsage],
    source_location: Option<&core::SourceLocation>,
) -> core::DiagnosticResult<(Vec<core::InputField>, Vec<core::ParamBinding>)> {
    if source_param_usages.len() != resolved_param_usages.len() {
        return Err(source_error(
            source_location,
            format!(
                "resolved Param usage count {} does not match source Param usage count {}",
                resolved_param_usages.len(),
                source_param_usages.len()
            ),
        ));
    }

    let mut input = Vec::<core::InputField>::new();
    let mut params = Vec::with_capacity(source_param_usages.len());

    for (source_usage, resolved_usage) in source_param_usages.iter().zip(resolved_param_usages) {
        if source_usage.id() != resolved_usage.id() {
            return Err(param_usage_error(
                source_location,
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
            if existing.type_ref() != resolved_usage.type_ref() {
                return Err(param_usage_error(
                    source_location,
                    source_usage,
                    format!(
                        "conflicting Param `{}` types: first occurrence resolved to {:?} but later occurrence resolved to {:?}",
                        source_usage.id(),
                        existing.type_ref(),
                        resolved_usage.type_ref()
                    ),
                ));
            }
            if existing.is_nullable() != nullable {
                return Err(param_usage_error(
                    source_location,
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
            input.push(core::InputField::new_type_ref(
                source_usage.id().to_owned(),
                resolved_usage.type_ref().clone(),
                nullable,
            ));
        }

        params.push(core::ParamBinding::new_type_ref(
            source_usage.id().to_owned(),
            resolved_usage.type_ref().clone(),
            nullable,
        ));
    }

    Ok((input, params))
}

fn source_error(
    source_location: Option<&core::SourceLocation>,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = source_location {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

fn param_usage_error(
    source_location: Option<&core::SourceLocation>,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            source_location
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    core::DiagnosticReport::new(core::Diagnostic::error(message).with_location(location))
}
