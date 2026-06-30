use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use sqlay_core as core;
use sqlparser::ast::{
    Assignment, AssignmentTarget, Delete, FromTable, Insert, ObjectName, OnInsert,
    Query as SqlQuery, SetExpr, Statement, TableFactor, TableObject, TableWithJoins, Update,
};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;

use super::super::diagnostics::{mutation_error, mutation_param_usage_error};
use super::super::schema_columns::MysqlSchemaColumn;
use super::super::schema_columns::MysqlSchemaTableRef;
use super::contexts::{ColumnRef, is_placeholder, join_constraint};
use super::mutation_contexts::{collect_mutation_expr_param_contexts, resolve_schema_column_type};
use super::tables::{TableResolution, object_name_parts, schema_table_ref_from_parts};
use super::unsupported_contexts::{
    collect_unsupported_expr_param_contexts, collect_unsupported_query_param_contexts,
};
use super::{SchemaColumnTypes, param_value_type_required_message};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MutationTableSources {
    by_qualifier: BTreeMap<String, TableResolution>,
    schema_table_refs: BTreeSet<MysqlSchemaTableRef>,
}

impl MutationTableSources {
    fn insert_resolution(&mut self, key: String, resolution: TableResolution) {
        match self.by_qualifier.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(resolution);
            }
            Entry::Occupied(mut entry) if entry.get() != &resolution => {
                entry.insert(TableResolution::Unsupported);
            }
            Entry::Occupied(_) => {}
        }
    }

    fn insert_schema_table(&mut self, table_ref: MysqlSchemaTableRef, alias: Option<String>) {
        self.schema_table_refs.insert(table_ref.clone());
        self.insert_resolution(
            table_ref.table_name().to_owned(),
            TableResolution::SchemaBacked {
                table_ref: table_ref.clone(),
            },
        );
        if let Some(qualifier_key) = table_ref.qualifier_key() {
            self.insert_resolution(
                qualifier_key,
                TableResolution::SchemaBacked {
                    table_ref: table_ref.clone(),
                },
            );
        }
        if let Some(alias) = alias {
            self.insert_resolution(alias, TableResolution::SchemaBacked { table_ref });
        }
    }

    fn insert_unsupported_table(&mut self, table_name: Option<String>, alias: Option<String>) {
        if let Some(table_name) = table_name {
            self.insert_resolution(table_name, TableResolution::Unsupported);
        }
        if let Some(alias) = alias {
            self.insert_resolution(alias, TableResolution::Unsupported);
        }
    }

    fn resolve(&self, qualifier: &str) -> Option<&TableResolution> {
        self.by_qualifier.get(qualifier)
    }
}

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
        let ty = if let Some(value_type) = usage.value_type_override() {
            value_type
        } else {
            resolve_inferred_mutation_param_type(
                mutation,
                usage,
                context.as_ref(),
                &table_sources,
                &schema,
            )?
        };
        params.push(core::DbParamUsage::new(usage.id().to_owned(), ty));
    }

    Ok(params)
}

fn resolve_inferred_mutation_param_type(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
    context: Option<&ColumnRef>,
    table_sources: &MutationTableSources,
    schema: &SchemaColumnTypes,
) -> core::DiagnosticResult<core::CoreType> {
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

    let Some(table) = table_sources.resolve(&column.qualifier) else {
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

    let TableResolution::SchemaBacked { table_ref } = table else {
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
    };

    resolve_schema_column_type(mutation, usage, table_ref, &column.column, schema)
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

fn collect_mutation_param_contexts(statement: &Statement) -> Vec<Option<ColumnRef>> {
    let mut contexts = Vec::new();

    match statement {
        Statement::Insert(insert) => collect_insert_param_contexts(insert, &mut contexts),
        Statement::Update(update) => collect_update_param_contexts(update, &mut contexts),
        Statement::Delete(delete) => collect_delete_param_contexts(delete, &mut contexts),
        _ => {}
    }

    contexts
}

fn collect_insert_param_contexts(insert: &Insert, contexts: &mut Vec<Option<ColumnRef>>) {
    let target_qualifier = insert_target_qualifier(insert);
    let target_qualifier = target_qualifier.as_deref();

    if let Some(source) = &insert.source {
        collect_insert_source_param_contexts(source, &insert.columns, target_qualifier, contexts);
    }
    for assignment in &insert.assignments {
        collect_assignment_param_context(assignment, target_qualifier, contexts);
    }
    if let Some(OnInsert::DuplicateKeyUpdate(assignments)) = &insert.on {
        for assignment in assignments {
            collect_assignment_param_context(assignment, target_qualifier, contexts);
        }
    }
}

fn collect_insert_source_param_contexts(
    query: &SqlQuery,
    columns: &[ObjectName],
    target_qualifier: Option<&str>,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_unsupported_query_param_contexts(&cte.query, contexts);
        }
    }

    collect_insert_set_expr_param_contexts(&query.body, columns, target_qualifier, contexts);
}

fn collect_insert_set_expr_param_contexts(
    expression: &SetExpr,
    columns: &[ObjectName],
    target_qualifier: Option<&str>,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match expression {
        SetExpr::Values(values) => {
            for row in &values.rows {
                for (index, expr) in row.iter().enumerate() {
                    if is_placeholder(expr) {
                        contexts.push(insert_column_context(columns.get(index), target_qualifier));
                    } else {
                        collect_mutation_expr_param_contexts(expr, contexts);
                    }
                }
            }
        }
        SetExpr::Query(query) => {
            collect_insert_source_param_contexts(query, columns, target_qualifier, contexts);
        }
        SetExpr::Select(_)
        | SetExpr::SetOperation { .. }
        | SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => {
            collect_unsupported_set_expr_param_contexts(expression, contexts);
        }
    }
}

fn collect_update_param_contexts(update: &Update, contexts: &mut Vec<Option<ColumnRef>>) {
    let target_qualifier = table_with_joins_default_qualifier(&update.table);
    let target_qualifier = target_qualifier.as_deref();

    for assignment in &update.assignments {
        collect_assignment_param_context(assignment, target_qualifier, contexts);
    }
    if let Some(selection) = &update.selection {
        collect_mutation_expr_param_contexts(selection, contexts);
    }
    for order_by in &update.order_by {
        collect_unsupported_expr_param_contexts(&order_by.expr, contexts);
    }
    if let Some(limit) = &update.limit {
        collect_unsupported_expr_param_contexts(limit, contexts);
    }
}

fn collect_delete_param_contexts(delete: &Delete, contexts: &mut Vec<Option<ColumnRef>>) {
    if let Some(selection) = &delete.selection {
        collect_mutation_expr_param_contexts(selection, contexts);
    }
    for order_by in &delete.order_by {
        collect_unsupported_expr_param_contexts(&order_by.expr, contexts);
    }
    if let Some(limit) = &delete.limit {
        collect_unsupported_expr_param_contexts(limit, contexts);
    }
}

fn collect_assignment_param_context(
    assignment: &Assignment,
    default_qualifier: Option<&str>,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if is_placeholder(&assignment.value) {
        contexts.push(assignment_target_column_context(
            &assignment.target,
            default_qualifier,
        ));
    } else {
        collect_mutation_expr_param_contexts(&assignment.value, contexts);
    }
}

fn collect_unsupported_set_expr_param_contexts(
    expression: &SetExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match expression {
        SetExpr::Select(select) => {
            for item in &select.projection {
                if let sqlparser::ast::SelectItem::UnnamedExpr(expr)
                | sqlparser::ast::SelectItem::ExprWithAlias { expr, .. }
                | sqlparser::ast::SelectItem::ExprWithAliases { expr, .. } = item
                {
                    collect_unsupported_expr_param_contexts(expr, contexts);
                }
            }
            for table in &select.from {
                collect_unsupported_table_with_joins_params(table, contexts);
            }
            if let Some(selection) = &select.selection {
                collect_unsupported_expr_param_contexts(selection, contexts);
            }
        }
        SetExpr::Query(query) => collect_unsupported_query_param_contexts(query, contexts),
        SetExpr::SetOperation { left, right, .. } => {
            collect_unsupported_set_expr_param_contexts(left, contexts);
            collect_unsupported_set_expr_param_contexts(right, contexts);
        }
        SetExpr::Values(values) => {
            for row in &values.rows {
                for expr in row.iter() {
                    collect_unsupported_expr_param_contexts(expr, contexts);
                }
            }
        }
        SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => {}
    }
}

fn collect_unsupported_table_with_joins_params(
    table: &TableWithJoins,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    collect_unsupported_table_factor_params(&table.relation, contexts);
    for join in &table.joins {
        collect_unsupported_table_factor_params(&join.relation, contexts);
        if let Some(constraint) = join_constraint(&join.join_operator)
            && let sqlparser::ast::JoinConstraint::On(expr) = constraint
        {
            collect_unsupported_expr_param_contexts(expr, contexts);
        }
    }
}

fn collect_unsupported_table_factor_params(
    table: &TableFactor,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match table {
        TableFactor::Derived { subquery, .. } => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        TableFactor::TableFunction { expr, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
        }
        TableFactor::JsonTable { json_expr, .. } => {
            collect_unsupported_expr_param_contexts(json_expr, contexts);
        }
        TableFactor::Function { args, .. } => {
            for arg in args {
                match arg {
                    sqlparser::ast::FunctionArg::Named { arg, .. }
                    | sqlparser::ast::FunctionArg::Unnamed(arg) => {
                        collect_unsupported_function_arg_params(arg, contexts);
                    }
                    sqlparser::ast::FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_unsupported_expr_param_contexts(name, contexts);
                        collect_unsupported_function_arg_params(arg, contexts);
                    }
                }
            }
        }
        TableFactor::UNNEST { array_exprs, .. } => {
            for expr in array_exprs {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            collect_unsupported_table_with_joins_params(table_with_joins, contexts);
        }
        _ => {}
    }
}

fn collect_unsupported_function_arg_params(
    arg: &sqlparser::ast::FunctionArgExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let sqlparser::ast::FunctionArgExpr::Expr(expr) = arg {
        collect_unsupported_expr_param_contexts(expr, contexts);
    }
}

fn insert_column_context(
    column: Option<&ObjectName>,
    target_qualifier: Option<&str>,
) -> Option<ColumnRef> {
    let column = column?;
    let qualifier = target_qualifier?;
    let parts = object_name_parts(column);
    if parts.iter().any(|part| part.contains('.')) {
        return None;
    }
    let column_name = parts.last().cloned()?;

    Some(ColumnRef {
        qualifier: qualifier.to_owned(),
        column: column_name,
        resolved_table_ref: None,
    })
}

fn assignment_target_column_context(
    target: &AssignmentTarget,
    default_qualifier: Option<&str>,
) -> Option<ColumnRef> {
    let AssignmentTarget::ColumnName(name) = target else {
        return None;
    };
    column_ref_from_object_name(name, default_qualifier)
}

fn column_ref_from_object_name(
    name: &ObjectName,
    default_qualifier: Option<&str>,
) -> Option<ColumnRef> {
    let parts = object_name_parts(name);
    match parts.as_slice() {
        [column] if !column.contains('.') => Some(ColumnRef::qualified(
            default_qualifier?.to_owned(),
            column.clone(),
        )),
        [qualifier, column] if !parts.iter().any(|part| part.contains('.')) => {
            Some(ColumnRef::qualified(qualifier.clone(), column.clone()))
        }
        [database, table, column] if !parts.iter().any(|part| part.contains('.')) => Some(
            ColumnRef::qualified(format!("{database}.{table}"), column.clone()),
        ),
        _ => None,
    }
}

fn mutation_table_sources(statement: &Statement) -> MutationTableSources {
    let mut sources = MutationTableSources::default();

    match statement {
        Statement::Insert(insert) => collect_insert_table_sources(insert, &mut sources),
        Statement::Update(update) => collect_table_with_joins_sources(&update.table, &mut sources),
        Statement::Delete(delete) => collect_delete_table_sources(delete, &mut sources),
        _ => {}
    }

    sources
}

fn collect_insert_table_sources(insert: &Insert, sources: &mut MutationTableSources) {
    let TableObject::TableName(name) = &insert.table else {
        return;
    };
    let alias = insert
        .table_alias
        .as_ref()
        .map(|alias| alias.alias.value.clone());
    collect_object_name_source(name, alias, sources);
}

fn collect_delete_table_sources(delete: &Delete, sources: &mut MutationTableSources) {
    match &delete.from {
        FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => {
            for table in tables {
                collect_table_with_joins_sources(table, sources);
            }
        }
    }
}

fn collect_table_with_joins_sources(table: &TableWithJoins, sources: &mut MutationTableSources) {
    collect_table_factor_source(&table.relation, sources);
    for join in &table.joins {
        collect_table_factor_source(&join.relation, sources);
    }
}

fn collect_table_factor_source(table: &TableFactor, sources: &mut MutationTableSources) {
    match table {
        TableFactor::Table {
            name, alias, args, ..
        } => {
            let alias = alias.as_ref().map(|alias| alias.name.value.clone());
            if args.is_none() {
                collect_object_name_source(name, alias, sources);
            } else {
                let parts = object_name_parts(name);
                sources.insert_unsupported_table(parts.last().cloned(), alias);
            }
        }
        TableFactor::Derived { alias, .. }
        | TableFactor::TableFunction { alias, .. }
        | TableFactor::Function { alias, .. }
        | TableFactor::JsonTable { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            collect_table_with_joins_sources(table_with_joins, sources);
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        _ => {}
    }
}

fn collect_object_name_source(
    name: &ObjectName,
    alias: Option<String>,
    sources: &mut MutationTableSources,
) {
    let parts = object_name_parts(name);
    if let Some(table_ref) = schema_table_ref_from_parts(&parts) {
        sources.insert_schema_table(table_ref, alias);
    } else {
        sources.insert_unsupported_table(parts.last().cloned(), alias);
    }
}

fn insert_target_qualifier(insert: &Insert) -> Option<String> {
    if let Some(alias) = &insert.table_alias {
        return Some(alias.alias.value.clone());
    }

    let TableObject::TableName(name) = &insert.table else {
        return None;
    };
    object_name_parts(name).last().cloned()
}

fn table_with_joins_default_qualifier(table: &TableWithJoins) -> Option<String> {
    let TableFactor::Table { name, alias, .. } = &table.relation else {
        return None;
    };
    alias
        .as_ref()
        .map(|alias| alias.name.value.clone())
        .or_else(|| object_name_parts(name).last().cloned())
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
