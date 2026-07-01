use sqlay_core as core;
use sqlparser::ast::{Expr, SelectItem};

use super::super::schema_columns::MysqlSchemaColumn;
use super::tables::{SelectTableSources, TableResolution, select_from_query, select_table_sources};
use super::{SchemaColumnTypes, parse_query, single_select_query};

pub(in crate::metadata::mysql::sqlx) fn resolve_result_column_type_refs(
    query: &core::RawQuery,
    schema_columns: &[MysqlSchemaColumn],
) -> core::DiagnosticResult<Vec<Option<core::CoreTypeRef>>> {
    if schema_columns.is_empty() {
        return Ok(Vec::new());
    }

    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    let table_sources = select_table_sources(parsed_query, select);
    let schema = SchemaColumnTypes::from_columns(schema_columns);
    let mut result_type_refs = Vec::with_capacity(select.projection.len());

    for item in &select.projection {
        let type_ref = match item {
            SelectItem::UnnamedExpr(expr)
            | SelectItem::ExprWithAlias { expr, .. }
            | SelectItem::ExprWithAliases { expr, .. } => {
                resolve_projection_expr_type_ref(expr, &table_sources, &schema)
            }
            SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => return Ok(Vec::new()),
        };
        result_type_refs.push(type_ref);
    }

    Ok(result_type_refs)
}

fn resolve_projection_expr_type_ref(
    expr: &Expr,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> Option<core::CoreTypeRef> {
    let (qualifier, column_name) = direct_projection_column_ref(expr)?;
    let Some(TableResolution::SchemaBacked { table_ref }) = table_sources.resolve(&qualifier)
    else {
        return None;
    };

    schema.get(table_ref, &column_name)
}

fn direct_projection_column_ref(expr: &Expr) -> Option<(String, String)> {
    match expr {
        Expr::CompoundIdentifier(parts) => match parts.as_slice() {
            [qualifier, column] => Some((qualifier.value.clone(), column.value.clone())),
            [database, table, column] => Some((
                format!("{}.{}", database.value, table.value),
                column.value.clone(),
            )),
            _ => None,
        },
        _ => None,
    }
}
