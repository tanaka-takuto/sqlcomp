use sqlparser::ast::{
    Assignment, AssignmentTarget, Delete, Insert, ObjectName, OnInsert, Query as SqlQuery, SetExpr,
    Statement, TableFactor, TableWithJoins, Update,
};

use super::super::contexts::{ColumnRef, is_placeholder, join_constraint};
use super::super::tables::object_name_parts;
use super::super::unsupported_contexts::{
    collect_unsupported_expr_param_contexts, collect_unsupported_query_param_contexts,
};
use super::param_contexts::collect_mutation_expr_param_contexts;
use super::table_sources::{insert_target_qualifier, table_with_joins_default_qualifier};

pub(super) fn collect_mutation_param_contexts(statement: &Statement) -> Vec<Option<ColumnRef>> {
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
