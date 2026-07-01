mod param_contexts;
mod param_usages;
mod table_sources;

use sqlay_core as core;
use sqlparser::ast::Statement;
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use self::param_contexts::resolve_schema_column_type;
use self::param_usages::collect_mutation_param_contexts;
use self::table_sources::{MutationTableSources, mutation_table_sources};
use super::super::diagnostics::{mutation_error, mutation_param_usage_error};
use super::super::schema_columns::{MysqlSchemaColumn, MysqlSchemaTableRef};
use super::contexts::ColumnRef;
use super::tables::TableResolution;
use super::{SchemaColumnTypes, param_value_type_required_message};

pub(in crate::metadata::mysql::sqlx) fn resolve_mutation_param_usage_metadata(
    mutation: &core::RawMutation,
    schema_columns: &[MysqlSchemaColumn],
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if mutation.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let statements = parse_mutation(mutation)?;
    let statement = single_mutation_statement(mutation, &statements)?;
    let mut contexts = collect_mutation_param_contexts(statement);
    if contexts.len() > mutation.param_usages().len() {
        return Err(mutation_error(
            mutation,
            format!(
                "resolved Param context count {} does not match source Param usage count {}",
                contexts.len(),
                mutation.param_usages().len()
            ),
        ));
    }
    contexts.resize(mutation.param_usages().len(), None);

    let table_sources = mutation_table_sources(statement);
    let schema = SchemaColumnTypes::from_columns(schema_columns);
    let mut params = Vec::with_capacity(mutation.param_usages().len());

    for (usage, context) in mutation.param_usages().iter().zip(contexts) {
        let type_ref = if let Some(value_type) = usage.value_type_override() {
            core::CoreTypeRef::from(value_type)
        } else {
            resolve_inferred_mutation_param_type(
                mutation,
                usage,
                context.as_ref(),
                &table_sources,
                &schema,
            )?
        };
        params.push(core::DbParamUsage::new_type_ref(
            usage.id().to_owned(),
            type_ref,
        ));
    }

    Ok(params)
}

fn resolve_inferred_mutation_param_type(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    context: Option<&ColumnRef>,
    table_sources: &MutationTableSources,
    schema: &SchemaColumnTypes,
) -> core::DiagnosticResult<core::CoreTypeRef> {
    let Some(column) = context else {
        return Err(mutation_param_usage_error(
            mutation,
            usage,
            param_value_type_required_message(
                usage.id(),
                "no supported mutation column context was found",
            ),
        ));
    };

    if let Some(table_ref) = column.resolved_table_ref.as_ref() {
        return resolve_schema_column_type(mutation, usage, table_ref, &column.column, schema);
    }

    let table_ref = match table_sources.resolve(&column.qualifier) {
        Some(TableResolution::SchemaBacked { table_ref }) => table_ref.clone(),
        Some(TableResolution::Unsupported) => {
            return Err(mutation_param_usage_error(
                mutation,
                usage,
                param_value_type_required_message(
                    usage.id(),
                    format!(
                        "table alias `{}` does not resolve to a supported schema-backed table",
                        column.qualifier
                    ),
                ),
            ));
        }
        None => {
            let Some(table_ref) = resolve_current_database_qualified_mutation_table_ref(
                table_sources,
                schema,
                &column.qualifier,
            ) else {
                return Err(mutation_param_usage_error(
                    mutation,
                    usage,
                    format!(
                        "Param `{}` references unknown table alias `{}`",
                        usage.id(),
                        column.qualifier
                    ),
                ));
            };
            table_ref
        }
    };

    resolve_schema_column_type(mutation, usage, &table_ref, &column.column, schema)
}

fn resolve_current_database_qualified_mutation_table_ref(
    table_sources: &MutationTableSources,
    schema: &SchemaColumnTypes,
    qualifier: &str,
) -> Option<MysqlSchemaTableRef> {
    let (database_name, table_name) = qualifier.split_once('.')?;
    let current_database_ref = MysqlSchemaTableRef::current_database(table_name.to_owned());
    let qualified_ref =
        MysqlSchemaTableRef::explicit_database(database_name.to_owned(), table_name.to_owned());
    if table_sources
        .schema_table_refs
        .contains(&current_database_ref)
        && schema.has_table(&qualified_ref)
    {
        return Some(qualified_ref);
    }

    let Some(TableResolution::SchemaBacked { table_ref }) = table_sources.resolve(table_name)
    else {
        return None;
    };
    if !table_ref.is_current_database() {
        return None;
    }

    schema.has_table(&qualified_ref).then_some(qualified_ref)
}

pub(in crate::metadata::mysql::sqlx) fn mutation_schema_table_refs(
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<Vec<MysqlSchemaTableRef>> {
    let statements = parse_mutation(mutation)?;
    let statement = single_mutation_statement(mutation, &statements)?;
    let mut table_refs = mutation_table_sources(statement).schema_table_refs;
    for context in collect_mutation_param_contexts(statement)
        .into_iter()
        .flatten()
    {
        if let Some(table_ref) = context.resolved_table_ref {
            table_refs.insert(table_ref);
        }
    }

    Ok(table_refs.into_iter().collect())
}

fn parse_mutation(mutation: &core::RawMutation) -> core::DiagnosticResult<Vec<Statement>> {
    let dialect = MySqlDialect {};
    Parser::parse_sql(&dialect, mutation.analysis_sql())
        .map_err(|error| mutation_error(mutation, format!("failed to parse MySQL SQL: {error}")))
}

fn single_mutation_statement<'a>(
    mutation: &core::RawMutation,
    statements: &'a [Statement],
) -> core::DiagnosticResult<&'a Statement> {
    let [statement] = statements else {
        return Err(mutation_error(
            mutation,
            "Param type inference requires exactly one mutation statement",
        ));
    };

    Ok(statement)
}
