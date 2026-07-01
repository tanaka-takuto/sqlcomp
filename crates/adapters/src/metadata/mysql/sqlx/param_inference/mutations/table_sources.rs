use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use sqlparser::ast::{
    Delete, FromTable, Insert, ObjectName, Statement, TableFactor, TableObject, TableWithJoins,
};

use super::super::super::schema_columns::MysqlSchemaTableRef;
use super::super::tables::{TableResolution, object_name_parts, schema_table_ref_from_parts};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MutationTableSources {
    by_qualifier: BTreeMap<String, TableResolution>,
    pub(super) schema_table_refs: BTreeSet<MysqlSchemaTableRef>,
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

    pub(super) fn resolve(&self, qualifier: &str) -> Option<&TableResolution> {
        self.by_qualifier.get(qualifier)
    }
}

pub(super) fn mutation_table_sources(statement: &Statement) -> MutationTableSources {
    let mut sources = MutationTableSources::default();

    match statement {
        Statement::Insert(insert) => collect_insert_table_sources(insert, &mut sources),
        Statement::Update(update) => collect_table_with_joins_sources(&update.table, &mut sources),
        Statement::Delete(delete) => collect_delete_table_sources(delete, &mut sources),
        _ => {}
    }

    sources
}

pub(super) fn insert_target_qualifier(insert: &Insert) -> Option<String> {
    if let Some(alias) = &insert.table_alias {
        return Some(alias.alias.value.clone());
    }

    let TableObject::TableName(name) = &insert.table else {
        return None;
    };
    object_name_parts(name).last().cloned()
}

pub(super) fn table_with_joins_default_qualifier(table: &TableWithJoins) -> Option<String> {
    let TableFactor::Table { name, alias, .. } = &table.relation else {
        return None;
    };
    alias
        .as_ref()
        .map(|alias| alias.name.value.clone())
        .or_else(|| object_name_parts(name).last().cloned())
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
