mod contexts;
mod mutation_contexts;
mod mutations;
mod tables;
mod unsupported_contexts;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use sqlay_core as core;
use sqlparser::ast::{Query as SqlQuery, Statement};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use super::diagnostics::{param_usage_error, query_error};
use super::schema_columns::MysqlSchemaColumn;
use contexts::{ColumnRef, collect_query_param_contexts};
pub(super) use mutations::{
    current_database_mutation_table_names, resolve_mutation_param_usage_metadata,
};
use tables::{SelectTableSources, TableResolution, select_from_query, select_table_sources};

const SUPPORTED_PARAM_VALUE_TYPES_MESSAGE: &str = "`bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SchemaColumnTypes {
    columns: BTreeMap<(String, String), core::CoreType>,
    tables: BTreeSet<String>,
}

impl SchemaColumnTypes {
    fn from_columns(columns: &[MysqlSchemaColumn]) -> Self {
        let mut schema = Self::default();
        for column in columns {
            schema.tables.insert(column.table_name.clone());
            schema.columns.insert(
                (column.table_name.clone(), column.column_name.clone()),
                column.ty,
            );
        }

        schema
    }

    fn get(&self, table_name: &str, column_name: &str) -> Option<core::CoreType> {
        self.columns
            .get(&(table_name.to_owned(), column_name.to_owned()))
            .copied()
    }

    fn has_table(&self, table_name: &str) -> bool {
        self.tables.contains(table_name)
    }
}

pub(super) fn resolve_param_usage_metadata(
    query: &core::RawQuery,
    schema_columns: &[MysqlSchemaColumn],
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if query.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    let mut contexts = collect_query_param_contexts(parsed_query, select);
    if contexts.len() > query.param_usages().len() {
        return Err(query_error(
            query,
            format!(
                "resolved Param context count {} does not match source Param usage count {}",
                contexts.len(),
                query.param_usages().len()
            ),
        ));
    }
    contexts.resize(query.param_usages().len(), None);

    let table_sources = select_table_sources(parsed_query, select);
    let schema = SchemaColumnTypes::from_columns(schema_columns);
    let mut params = Vec::with_capacity(query.param_usages().len());

    for (usage, context) in query.param_usages().iter().zip(contexts) {
        let ty = if let Some(value_type) = usage.value_type_override() {
            value_type
        } else {
            resolve_inferred_param_type(query, usage, context.as_ref(), &table_sources, &schema)?
        };
        params.push(core::DbParamUsage::new(usage.id().to_owned(), ty));
    }

    Ok(params)
}

fn resolve_inferred_param_type(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    context: Option<&ColumnRef>,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> core::DiagnosticResult<core::CoreType> {
    let Some(column) = context else {
        return Err(param_usage_error(
            query,
            usage,
            param_value_type_required_message(
                usage.id(),
                "no supported qualified column context was found",
            ),
        ));
    };

    let Some(table) = table_sources.resolve(&column.qualifier) else {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` references unknown table alias `{}`",
                usage.id(),
                column.qualifier
            ),
        ));
    };

    let TableResolution::CurrentDatabase { table_name } = table else {
        return Err(param_usage_error(
            query,
            usage,
            param_value_type_required_message(
                usage.id(),
                format!(
                    "table alias `{}` does not resolve to a current-database table",
                    column.qualifier
                ),
            ),
        ));
    };

    if let Some(ty) = schema.get(table_name, &column.column) {
        return Ok(ty);
    }

    if !schema.has_table(table_name) {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` references unknown current-database table `{table_name}`",
                usage.id()
            ),
        ));
    }

    Err(param_usage_error(
        query,
        usage,
        format!(
            "Param `{}` references unknown current-database column `{table_name}.{}`",
            usage.id(),
            column.column
        ),
    ))
}

fn param_value_type_required_message(id: &str, reason: impl AsRef<str>) -> String {
    let reason = reason.as_ref();
    format!(
        "Param `{id}` requires `valueType` because {reason}; use an inline `valueType` such as `valueType: string` or compare the Param directly with a qualified column; supported values are {SUPPORTED_PARAM_VALUE_TYPES_MESSAGE}; use `nullable: true` for nullable inputs"
    )
}

pub(super) fn current_database_table_names(
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<String>> {
    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    Ok(select_table_sources(parsed_query, select)
        .current_database_table_names
        .into_iter()
        .collect())
}

fn parse_query(query: &core::RawQuery) -> core::DiagnosticResult<Vec<Statement>> {
    let dialect = MySqlDialect {};
    Parser::parse_sql(&dialect, query.analysis_sql())
        .map_err(|error| query_error(query, format!("failed to parse MySQL SQL: {error}")))
}

fn single_select_query<'a>(
    query: &core::RawQuery,
    statements: &'a [Statement],
) -> core::DiagnosticResult<&'a SqlQuery> {
    let [Statement::Query(parsed_query)] = statements else {
        return Err(query_error(
            query,
            "Param type inference requires exactly one SELECT query",
        ));
    };

    if select_from_query(parsed_query).is_none() {
        return Err(query_error(
            query,
            "Param type inference requires a top-level SELECT query",
        ));
    }

    Ok(parsed_query)
}
