use sqlay_core as core;
use sqlparser::ast::{Expr, Query as SqlQuery};

use super::super::diagnostics::mutation_param_usage_error;
use super::super::schema_columns::MysqlSchemaTableRef;
use super::SchemaColumnTypes;
use super::contexts::{
    ColumnRef, collect_expr_param_contexts_with_query_handler, collect_query_param_contexts,
};
use super::tables::{SelectTableSources, TableResolution, select_from_query, select_table_sources};
use super::unsupported_contexts::collect_unsupported_query_param_contexts;

pub(super) fn collect_mutation_expr_param_contexts(
    expr: &Expr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    collect_expr_param_contexts_with_query_handler(
        expr,
        contexts,
        &mut collect_select_subquery_param_contexts,
    );
}

pub(super) fn resolve_schema_column_type(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    table_ref: &MysqlSchemaTableRef,
    column_name: &str,
    schema: &SchemaColumnTypes,
) -> core::DiagnosticResult<core::CoreTypeRef> {
    if let Some(type_ref) = schema.get(table_ref, column_name) {
        return Ok(type_ref);
    }

    if !schema.has_table(table_ref) {
        return Err(mutation_param_usage_error(
            mutation,
            usage,
            format!(
                "Param `{}` references unknown {}",
                usage.id(),
                table_ref.table_description()
            ),
        ));
    }

    Err(mutation_param_usage_error(
        mutation,
        usage,
        table_ref.unknown_column_message(usage.id(), column_name),
    ))
}

fn collect_select_subquery_param_contexts(query: &SqlQuery, contexts: &mut Vec<Option<ColumnRef>>) {
    let Some(select) = select_from_query(query) else {
        collect_unsupported_query_param_contexts(query, contexts);
        return;
    };

    let table_sources = select_table_sources(query, select);
    let subquery_contexts = collect_query_param_contexts(query, select);
    contexts.extend(
        subquery_contexts
            .into_iter()
            .map(|context| resolve_select_subquery_column_context(context, &table_sources)),
    );
}

fn resolve_select_subquery_column_context(
    context: Option<ColumnRef>,
    table_sources: &SelectTableSources,
) -> Option<ColumnRef> {
    let column = context?;
    match table_sources.resolve(&column.qualifier) {
        Some(TableResolution::SchemaBacked { table_ref }) => Some(
            ColumnRef::resolved_schema_table(column.qualifier, table_ref.clone(), column.column),
        ),
        Some(TableResolution::Unsupported) | None => None,
    }
}
