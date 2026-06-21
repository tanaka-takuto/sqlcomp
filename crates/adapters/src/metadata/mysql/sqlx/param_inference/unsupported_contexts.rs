use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr, JoinConstraint,
    LimitClause, OrderBy, OrderByKind, Query as SqlQuery, Select, SelectItem, SetExpr, TableFactor,
    TableWithJoins,
};

use super::contexts::{ColumnRef, is_placeholder, join_constraint};

pub(super) fn collect_unsupported_query_param_contexts(
    query: &SqlQuery,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_unsupported_query_param_contexts(&cte.query, contexts);
        }
    }
    collect_unsupported_set_expr_param_contexts(&query.body, contexts);
    if let Some(order_by) = &query.order_by {
        collect_unsupported_order_by_param_contexts(order_by, contexts);
    }
    if let Some(limit_clause) = &query.limit_clause {
        collect_unsupported_limit_clause_param_contexts(limit_clause, contexts);
    }
}

fn collect_unsupported_set_expr_param_contexts(
    expression: &SetExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match expression {
        SetExpr::Select(select) => collect_unsupported_select_param_contexts(select, contexts),
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

fn collect_unsupported_select_param_contexts(
    select: &Select,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    for item in &select.projection {
        collect_unsupported_select_item_param_contexts(item, contexts);
    }
    for table in &select.from {
        collect_unsupported_table_with_joins_param_contexts(table, contexts);
    }
    if let Some(selection) = &select.selection {
        collect_unsupported_expr_param_contexts(selection, contexts);
    }
    collect_unsupported_group_by_param_contexts(&select.group_by, contexts);
    if let Some(having) = &select.having {
        collect_unsupported_expr_param_contexts(having, contexts);
    }
}

fn collect_unsupported_select_item_param_contexts(
    item: &SelectItem,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match item {
        SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
        }
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => {}
    }
}

fn collect_unsupported_table_with_joins_param_contexts(
    table: &TableWithJoins,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    collect_unsupported_table_factor_param_contexts(&table.relation, contexts);
    for join in &table.joins {
        collect_unsupported_table_factor_param_contexts(&join.relation, contexts);
        if let Some(constraint) = join_constraint(&join.join_operator) {
            collect_unsupported_join_constraint_param_contexts(constraint, contexts);
        }
    }
}

fn collect_unsupported_table_factor_param_contexts(
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
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_unsupported_expr_param_contexts(name, contexts);
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
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
            collect_unsupported_table_with_joins_param_contexts(table_with_joins, contexts);
        }
        _ => {}
    }
}

fn collect_unsupported_join_constraint_param_contexts(
    constraint: &JoinConstraint,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let JoinConstraint::On(expr) = constraint {
        collect_unsupported_expr_param_contexts(expr, contexts);
    }
}

fn collect_unsupported_group_by_param_contexts(
    group_by: &GroupByExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match group_by {
        GroupByExpr::Expressions(expressions, _) => {
            for expr in expressions {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        GroupByExpr::All(_) => {}
    }
}

fn collect_unsupported_order_by_param_contexts(
    order_by: &OrderBy,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match &order_by.kind {
        OrderByKind::Expressions(expressions) => {
            for order_by_expr in expressions {
                collect_unsupported_expr_param_contexts(&order_by_expr.expr, contexts);
                if let Some(with_fill) = &order_by_expr.with_fill {
                    if let Some(from) = &with_fill.from {
                        collect_unsupported_expr_param_contexts(from, contexts);
                    }
                    if let Some(to) = &with_fill.to {
                        collect_unsupported_expr_param_contexts(to, contexts);
                    }
                    if let Some(step) = &with_fill.step {
                        collect_unsupported_expr_param_contexts(step, contexts);
                    }
                }
            }
        }
        OrderByKind::All(_) => {}
    }
    if let Some(interpolate) = &order_by.interpolate
        && let Some(expressions) = &interpolate.exprs
    {
        for expr in expressions {
            if let Some(expr) = &expr.expr {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
    }
}

fn collect_unsupported_limit_clause_param_contexts(
    limit_clause: &LimitClause,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match limit_clause {
        LimitClause::LimitOffset {
            limit,
            offset,
            limit_by,
        } => {
            if let Some(limit) = limit {
                collect_unsupported_expr_param_contexts(limit, contexts);
            }
            if let Some(offset) = offset {
                collect_unsupported_expr_param_contexts(&offset.value, contexts);
            }
            for expr in limit_by {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        LimitClause::OffsetCommaLimit { offset, limit } => {
            collect_unsupported_expr_param_contexts(offset, contexts);
            collect_unsupported_expr_param_contexts(limit, contexts);
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn collect_unsupported_expr_param_contexts(
    expr: &Expr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if is_placeholder(expr) {
        contexts.push(None);
        return;
    }

    match expr {
        Expr::BinaryOp { left, right, .. }
        | Expr::AnyOp { left, right, .. }
        | Expr::AllOp { left, right, .. }
        | Expr::IsDistinctFrom(left, right)
        | Expr::IsNotDistinctFrom(left, right) => {
            collect_unsupported_expr_param_contexts(left, contexts);
            collect_unsupported_expr_param_contexts(right, contexts);
        }
        Expr::InList { expr, list, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            for item in list {
                collect_unsupported_expr_param_contexts(item, contexts);
            }
        }
        Expr::InSubquery { expr, subquery, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::InUnnest {
            expr, array_expr, ..
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(array_expr, contexts);
        }
        Expr::Nested(expr)
        | Expr::UnaryOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Extract { expr, .. }
        | Expr::Ceil { expr, .. }
        | Expr::Floor { expr, .. }
        | Expr::Collate { expr, .. }
        | Expr::Prefixed { value: expr, .. }
        | Expr::IsFalse(expr)
        | Expr::IsNotFalse(expr)
        | Expr::IsTrue(expr)
        | Expr::IsNotTrue(expr)
        | Expr::IsNull(expr)
        | Expr::IsNotNull(expr)
        | Expr::IsUnknown(expr)
        | Expr::IsNotUnknown(expr)
        | Expr::OuterJoin(expr)
        | Expr::Prior(expr)
        | Expr::Named { expr, .. } => collect_unsupported_expr_param_contexts(expr, contexts),
        Expr::Between {
            expr, low, high, ..
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(low, contexts);
            collect_unsupported_expr_param_contexts(high, contexts);
        }
        Expr::Like { expr, pattern, .. }
        | Expr::ILike { expr, pattern, .. }
        | Expr::SimilarTo { expr, pattern, .. }
        | Expr::RLike { expr, pattern, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(pattern, contexts);
        }
        Expr::Convert { expr, styles, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            for style in styles {
                collect_unsupported_expr_param_contexts(style, contexts);
            }
        }
        Expr::AtTimeZone {
            timestamp,
            time_zone,
        } => {
            collect_unsupported_expr_param_contexts(timestamp, contexts);
            collect_unsupported_expr_param_contexts(time_zone, contexts);
        }
        Expr::Position { expr, r#in } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(r#in, contexts);
        }
        Expr::Substring {
            expr,
            substring_from,
            substring_for,
            ..
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            if let Some(substring_from) = substring_from {
                collect_unsupported_expr_param_contexts(substring_from, contexts);
            }
            if let Some(substring_for) = substring_for {
                collect_unsupported_expr_param_contexts(substring_for, contexts);
            }
        }
        Expr::Trim {
            trim_what,
            expr,
            trim_characters,
            ..
        } => {
            if let Some(trim_what) = trim_what {
                collect_unsupported_expr_param_contexts(trim_what, contexts);
            }
            collect_unsupported_expr_param_contexts(expr, contexts);
            if let Some(trim_characters) = trim_characters {
                for character in trim_characters {
                    collect_unsupported_expr_param_contexts(character, contexts);
                }
            }
        }
        Expr::Overlay {
            expr,
            overlay_what,
            overlay_from,
            overlay_for,
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(overlay_what, contexts);
            collect_unsupported_expr_param_contexts(overlay_from, contexts);
            if let Some(overlay_for) = overlay_for {
                collect_unsupported_expr_param_contexts(overlay_for, contexts);
            }
        }
        Expr::Function(function) => {
            collect_unsupported_function_arguments_param_contexts(&function.parameters, contexts);
            collect_unsupported_function_arguments_param_contexts(&function.args, contexts);
            if let Some(filter) = &function.filter {
                collect_unsupported_expr_param_contexts(filter, contexts);
            }
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            if let Some(operand) = operand {
                collect_unsupported_expr_param_contexts(operand, contexts);
            }
            for condition in conditions {
                collect_unsupported_expr_param_contexts(&condition.condition, contexts);
                collect_unsupported_expr_param_contexts(&condition.result, contexts);
            }
            if let Some(else_result) = else_result {
                collect_unsupported_expr_param_contexts(else_result, contexts);
            }
        }
        Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::GroupingSets(items) | Expr::Cube(items) | Expr::Rollup(items) => {
            for item in items {
                for expr in item {
                    collect_unsupported_expr_param_contexts(expr, contexts);
                }
            }
        }
        Expr::Tuple(items) => {
            for item in items {
                collect_unsupported_expr_param_contexts(item, contexts);
            }
        }
        Expr::Struct { values, .. } => {
            for value in values {
                collect_unsupported_expr_param_contexts(value, contexts);
            }
        }
        _ => {}
    }
}

fn collect_unsupported_function_arguments_param_contexts(
    arguments: &FunctionArguments,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match arguments {
        FunctionArguments::None => {}
        FunctionArguments::Subquery(query) => {
            collect_unsupported_query_param_contexts(query, contexts);
        }
        FunctionArguments::List(list) => {
            for arg in &list.args {
                match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_unsupported_expr_param_contexts(name, contexts);
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                }
            }
        }
    }
}

pub(super) fn collect_unsupported_function_arg_expr_param_contexts(
    arg: &FunctionArgExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let FunctionArgExpr::Expr(expr) = arg {
        collect_unsupported_expr_param_contexts(expr, contexts);
    }
}
