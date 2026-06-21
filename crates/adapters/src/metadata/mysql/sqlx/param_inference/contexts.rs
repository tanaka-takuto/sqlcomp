use sqlparser::ast::{
    BinaryOperator, Expr, FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr,
    JoinConstraint, JoinOperator, LimitClause, OrderBy, OrderByKind, Query as SqlQuery, Select,
    SelectItem, TableFactor, TableWithJoins, Value,
};

use super::unsupported_contexts::{
    collect_unsupported_expr_param_contexts, collect_unsupported_function_arg_expr_param_contexts,
    collect_unsupported_query_param_contexts,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ColumnRef {
    pub(super) qualifier: String,
    pub(super) column: String,
}

pub(super) fn collect_query_param_contexts(
    query: &SqlQuery,
    select: &Select,
) -> Vec<Option<ColumnRef>> {
    let mut contexts = Vec::new();

    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_unsupported_query_param_contexts(&cte.query, &mut contexts);
        }
    }
    for item in &select.projection {
        collect_select_item_param_contexts(item, &mut contexts);
    }
    for table in &select.from {
        collect_table_with_joins_param_contexts(table, &mut contexts);
    }
    if let Some(selection) = &select.selection {
        collect_expr_param_contexts(selection, &mut contexts);
    }
    collect_group_by_param_contexts(&select.group_by, &mut contexts);
    if let Some(having) = &select.having {
        collect_expr_param_contexts(having, &mut contexts);
    }
    if let Some(order_by) = &query.order_by {
        collect_order_by_param_contexts(order_by, &mut contexts);
    }
    if let Some(limit_clause) = &query.limit_clause {
        collect_limit_clause_param_contexts(limit_clause, &mut contexts);
    }

    contexts
}

fn collect_select_item_param_contexts(item: &SelectItem, contexts: &mut Vec<Option<ColumnRef>>) {
    match item {
        SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } => collect_expr_param_contexts(expr, contexts),
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => {}
    }
}

fn collect_table_with_joins_param_contexts(
    table: &TableWithJoins,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    collect_table_factor_param_contexts(&table.relation, contexts);
    for join in &table.joins {
        collect_table_factor_param_contexts(&join.relation, contexts);
        if let Some(constraint) = join_constraint(&join.join_operator) {
            collect_join_constraint_param_contexts(constraint, contexts);
        }
    }
}

fn collect_table_factor_param_contexts(table: &TableFactor, contexts: &mut Vec<Option<ColumnRef>>) {
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
            collect_table_with_joins_param_contexts(table_with_joins, contexts);
        }
        _ => {}
    }
}

pub(super) const fn join_constraint(operator: &JoinOperator) -> Option<&JoinConstraint> {
    match operator {
        JoinOperator::Join(constraint)
        | JoinOperator::Inner(constraint)
        | JoinOperator::Left(constraint)
        | JoinOperator::LeftOuter(constraint)
        | JoinOperator::Right(constraint)
        | JoinOperator::RightOuter(constraint)
        | JoinOperator::FullOuter(constraint)
        | JoinOperator::CrossJoin(constraint)
        | JoinOperator::Semi(constraint)
        | JoinOperator::LeftSemi(constraint)
        | JoinOperator::RightSemi(constraint)
        | JoinOperator::Anti(constraint)
        | JoinOperator::LeftAnti(constraint)
        | JoinOperator::RightAnti(constraint)
        | JoinOperator::StraightJoin(constraint)
        | JoinOperator::AsOf { constraint, .. } => Some(constraint),
        JoinOperator::CrossApply
        | JoinOperator::OuterApply
        | JoinOperator::ArrayJoin
        | JoinOperator::LeftArrayJoin
        | JoinOperator::InnerArrayJoin => None,
    }
}

fn collect_join_constraint_param_contexts(
    constraint: &JoinConstraint,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let JoinConstraint::On(expr) = constraint {
        collect_expr_param_contexts(expr, contexts);
    }
}

fn collect_group_by_param_contexts(group_by: &GroupByExpr, contexts: &mut Vec<Option<ColumnRef>>) {
    match group_by {
        GroupByExpr::Expressions(expressions, _) => {
            for expr in expressions {
                collect_expr_param_contexts(expr, contexts);
            }
        }
        GroupByExpr::All(_) => {}
    }
}

fn collect_order_by_param_contexts(order_by: &OrderBy, contexts: &mut Vec<Option<ColumnRef>>) {
    match &order_by.kind {
        OrderByKind::Expressions(expressions) => {
            for order_by_expr in expressions {
                collect_expr_param_contexts(&order_by_expr.expr, contexts);
                if let Some(with_fill) = &order_by_expr.with_fill {
                    if let Some(from) = &with_fill.from {
                        collect_expr_param_contexts(from, contexts);
                    }
                    if let Some(to) = &with_fill.to {
                        collect_expr_param_contexts(to, contexts);
                    }
                    if let Some(step) = &with_fill.step {
                        collect_expr_param_contexts(step, contexts);
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
                collect_expr_param_contexts(expr, contexts);
            }
        }
    }
}

fn collect_limit_clause_param_contexts(
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
                collect_expr_param_contexts(limit, contexts);
            }
            if let Some(offset) = offset {
                collect_expr_param_contexts(&offset.value, contexts);
            }
            for expr in limit_by {
                collect_expr_param_contexts(expr, contexts);
            }
        }
        LimitClause::OffsetCommaLimit { offset, limit } => {
            collect_expr_param_contexts(offset, contexts);
            collect_expr_param_contexts(limit, contexts);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn collect_expr_param_contexts(expr: &Expr, contexts: &mut Vec<Option<ColumnRef>>) {
    if is_placeholder(expr) {
        contexts.push(None);
        return;
    }

    match expr {
        Expr::BinaryOp { left, op, right } if is_supported_comparison_operator(op) => {
            match (qualified_column_ref(left), is_placeholder(right)) {
                (Some(column), true) => contexts.push(Some(column)),
                _ => {
                    if let (true, Some(column)) =
                        (is_placeholder(left), qualified_column_ref(right))
                    {
                        contexts.push(Some(column));
                    } else {
                        collect_expr_param_contexts(left, contexts);
                        collect_expr_param_contexts(right, contexts);
                    }
                }
            }
        }
        Expr::BinaryOp { left, right, .. }
        | Expr::AnyOp { left, right, .. }
        | Expr::AllOp { left, right, .. }
        | Expr::IsDistinctFrom(left, right)
        | Expr::IsNotDistinctFrom(left, right) => {
            collect_expr_param_contexts(left, contexts);
            collect_expr_param_contexts(right, contexts);
        }
        Expr::InList {
            expr,
            list,
            negated,
        } if !negated => {
            if let Some(column) = qualified_column_ref(expr) {
                for item in list {
                    if is_placeholder(item) {
                        contexts.push(Some(column.clone()));
                    } else {
                        collect_expr_param_contexts(item, contexts);
                    }
                }
            } else {
                collect_expr_param_contexts(expr, contexts);
                for item in list {
                    collect_expr_param_contexts(item, contexts);
                }
            }
        }
        Expr::InList { expr, list, .. } => {
            collect_expr_param_contexts(expr, contexts);
            for item in list {
                collect_expr_param_contexts(item, contexts);
            }
        }
        Expr::InSubquery { expr, subquery, .. } => {
            collect_expr_param_contexts(expr, contexts);
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::InUnnest {
            expr, array_expr, ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(array_expr, contexts);
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
        | Expr::Named { expr, .. } => collect_expr_param_contexts(expr, contexts),
        Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::Between {
            expr, low, high, ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(low, contexts);
            collect_expr_param_contexts(high, contexts);
        }
        Expr::Like { expr, pattern, .. }
        | Expr::ILike { expr, pattern, .. }
        | Expr::SimilarTo { expr, pattern, .. }
        | Expr::RLike { expr, pattern, .. } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(pattern, contexts);
        }
        Expr::Convert { expr, styles, .. } => {
            collect_expr_param_contexts(expr, contexts);
            for style in styles {
                collect_expr_param_contexts(style, contexts);
            }
        }
        Expr::AtTimeZone {
            timestamp,
            time_zone,
        } => {
            collect_expr_param_contexts(timestamp, contexts);
            collect_expr_param_contexts(time_zone, contexts);
        }
        Expr::Position { expr, r#in } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(r#in, contexts);
        }
        Expr::Substring {
            expr,
            substring_from,
            substring_for,
            ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            if let Some(substring_from) = substring_from {
                collect_expr_param_contexts(substring_from, contexts);
            }
            if let Some(substring_for) = substring_for {
                collect_expr_param_contexts(substring_for, contexts);
            }
        }
        Expr::Trim {
            trim_what,
            expr,
            trim_characters,
            ..
        } => {
            if let Some(trim_what) = trim_what {
                collect_expr_param_contexts(trim_what, contexts);
            }
            collect_expr_param_contexts(expr, contexts);
            if let Some(trim_characters) = trim_characters {
                for character in trim_characters {
                    collect_expr_param_contexts(character, contexts);
                }
            }
        }
        Expr::Overlay {
            expr,
            overlay_what,
            overlay_from,
            overlay_for,
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(overlay_what, contexts);
            collect_expr_param_contexts(overlay_from, contexts);
            if let Some(overlay_for) = overlay_for {
                collect_expr_param_contexts(overlay_for, contexts);
            }
        }
        Expr::Function(function) => {
            collect_function_arguments_param_contexts(&function.parameters, contexts);
            collect_function_arguments_param_contexts(&function.args, contexts);
            if let Some(filter) = &function.filter {
                collect_expr_param_contexts(filter, contexts);
            }
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            if let Some(operand) = operand {
                collect_expr_param_contexts(operand, contexts);
            }
            for condition in conditions {
                collect_expr_param_contexts(&condition.condition, contexts);
                collect_expr_param_contexts(&condition.result, contexts);
            }
            if let Some(else_result) = else_result {
                collect_expr_param_contexts(else_result, contexts);
            }
        }
        Expr::GroupingSets(items) | Expr::Cube(items) | Expr::Rollup(items) => {
            for item in items {
                for expr in item {
                    collect_expr_param_contexts(expr, contexts);
                }
            }
        }
        Expr::Tuple(items) => {
            for item in items {
                collect_expr_param_contexts(item, contexts);
            }
        }
        Expr::Struct { values, .. } => {
            for value in values {
                collect_expr_param_contexts(value, contexts);
            }
        }
        _ => {}
    }
}

fn collect_function_arguments_param_contexts(
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
                        collect_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_expr_param_contexts(name, contexts);
                        collect_function_arg_expr_param_contexts(arg, contexts);
                    }
                }
            }
        }
    }
}

fn collect_function_arg_expr_param_contexts(
    arg: &FunctionArgExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let FunctionArgExpr::Expr(expr) = arg {
        collect_expr_param_contexts(expr, contexts);
    }
}

const fn is_supported_comparison_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::LtEq
            | BinaryOperator::Gt
            | BinaryOperator::GtEq
    )
}

fn qualified_column_ref(expr: &Expr) -> Option<ColumnRef> {
    let Expr::CompoundIdentifier(parts) = expr else {
        return None;
    };
    let [qualifier, column] = parts.as_slice() else {
        return None;
    };

    Some(ColumnRef {
        qualifier: qualifier.value.clone(),
        column: column.value.clone(),
    })
}

pub(super) fn is_placeholder(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(value) if matches!(&value.value, Value::Placeholder(value) if value == "?"))
}
