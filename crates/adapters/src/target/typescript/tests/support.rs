use std::path::{Path, PathBuf};

use sqlay_core as core;

pub(super) fn compilation_plan() -> core::CompilationPlan {
    core::CompilationPlan::new(
        PathBuf::from("/tmp/sqlay-project"),
        vec![PathBuf::from("/tmp/sqlay-project/sql/**/*.sql")],
        Vec::new(),
        PathBuf::from("/tmp/sqlay-project/src/generated/sqlay"),
        core::DatabaseConfig::new(core::DatabaseDialect::MySql, "DATABASE_URL".to_owned()),
        core::TargetConfig::new(core::TargetLanguage::TypeScript),
    )
}

pub(super) fn compiled_query(id: &str, sql: &str) -> core::CompiledQuery {
    core::CompiledQuery::new(
        core::QueryId::new(id.to_owned()),
        sql.to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int32,
            false,
        )],
    )
}

pub(super) fn slot_definition(
    id: &str,
    branches: Vec<core::CompiledSlotBranch>,
) -> core::CompiledSlotDefinition {
    core::CompiledSlotDefinition::new(id.to_owned(), branches)
}

pub(super) fn slot_branch(
    target_id: &str,
    sql: &str,
    params: Vec<core::ParamBinding>,
) -> core::CompiledSlotBranch {
    core::CompiledSlotBranch::new(target_id.to_owned(), vec![sql_segment(sql, params)])
}

pub(super) fn sql_segment(sql: &str, params: Vec<core::ParamBinding>) -> core::CompiledSqlSegment {
    core::CompiledSqlSegment::new(sql.to_owned(), params)
}

pub(super) fn param(name: &str, ty: core::CoreType, nullable: bool) -> core::ParamBinding {
    core::ParamBinding::new(name.to_owned(), ty, nullable)
}

pub(super) fn file_contents<'a>(files: &'a core::GeneratedFiles, path: &Path) -> &'a str {
    files
        .files()
        .iter()
        .find(|file| file.path() == path)
        .unwrap_or_else(|| panic!("expected generated file `{}`", path.display()))
        .contents()
}
