use std::collections::{BTreeMap, BTreeSet};

use sqlparser::ast::{ObjectName, Query as SqlQuery, Select, SetExpr, TableFactor, TableWithJoins};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum TableResolution {
    CurrentDatabase { table_name: String },
    Unsupported,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct SelectTableSources {
    by_qualifier: BTreeMap<String, TableResolution>,
    pub(super) current_database_table_names: BTreeSet<String>,
}

impl SelectTableSources {
    fn insert_current_database_table(&mut self, table_name: String, alias: Option<String>) {
        self.current_database_table_names.insert(table_name.clone());
        self.by_qualifier.insert(
            table_name.clone(),
            TableResolution::CurrentDatabase {
                table_name: table_name.clone(),
            },
        );
        if let Some(alias) = alias {
            self.by_qualifier
                .insert(alias, TableResolution::CurrentDatabase { table_name });
        }
    }

    fn insert_unsupported_table(&mut self, table_name: Option<String>, alias: Option<String>) {
        if let Some(table_name) = table_name {
            self.by_qualifier
                .insert(table_name, TableResolution::Unsupported);
        }
        if let Some(alias) = alias {
            self.by_qualifier
                .insert(alias, TableResolution::Unsupported);
        }
    }

    pub(super) fn resolve(&self, qualifier: &str) -> Option<&TableResolution> {
        self.by_qualifier.get(qualifier)
    }
}

pub(super) fn select_table_sources(query: &SqlQuery, select: &Select) -> SelectTableSources {
    let cte_names = cte_names(query);
    let mut sources = SelectTableSources::default();
    for table in &select.from {
        collect_table_with_joins_sources(table, &mut sources, &cte_names);
    }

    sources
}

pub(super) fn select_from_query(query: &SqlQuery) -> Option<&Select> {
    match query.body.as_ref() {
        SetExpr::Select(select) => Some(select),
        SetExpr::Query(query) => select_from_query(query),
        SetExpr::SetOperation { .. }
        | SetExpr::Values(_)
        | SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => None,
    }
}

fn cte_names(query: &SqlQuery) -> BTreeSet<String> {
    query
        .with
        .as_ref()
        .map(|with| {
            with.cte_tables
                .iter()
                .map(|cte| cte.alias.name.value.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn collect_table_with_joins_sources(
    table: &TableWithJoins,
    sources: &mut SelectTableSources,
    cte_names: &BTreeSet<String>,
) {
    collect_table_factor_source(&table.relation, sources, cte_names);
    for join in &table.joins {
        collect_table_factor_source(&join.relation, sources, cte_names);
    }
}

fn collect_table_factor_source(
    table: &TableFactor,
    sources: &mut SelectTableSources,
    cte_names: &BTreeSet<String>,
) {
    match table {
        TableFactor::Table {
            name, alias, args, ..
        } => {
            let alias = alias.as_ref().map(|alias| alias.name.value.clone());
            let parts = object_name_parts(name);
            if parts.len() == 1 && cte_names.contains(&parts[0]) {
                sources.insert_unsupported_table(Some(parts[0].clone()), alias);
            } else if args.is_none() && parts.len() == 1 {
                sources.insert_current_database_table(parts[0].clone(), alias);
            } else {
                sources.insert_unsupported_table(parts.last().cloned(), alias);
            }
        }
        TableFactor::Derived { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::TableFunction { alias, .. } | TableFactor::Function { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::JsonTable { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            collect_table_with_joins_sources(table_with_joins, sources, cte_names);
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        _ => {}
    }
}

fn object_name_parts(name: &ObjectName) -> Vec<String> {
    name.0
        .iter()
        .filter_map(|part| part.as_ident().map(|ident| ident.value.clone()))
        .collect()
}
