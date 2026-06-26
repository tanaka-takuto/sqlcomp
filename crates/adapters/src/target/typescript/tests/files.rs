use std::path::Path;

use sqlay_app::TargetGenerator;
use sqlay_core as core;

use super::super::TypeScriptTargetGenerator;
use super::support::{
    compilation_plan, compiled_query, file_contents, param, slot_branch, slot_definition,
    sql_segment,
};

#[test]
fn generator_keeps_slotless_files_on_static_builder_surface_when_slots_are_compiled_elsewhere() {
    let plan = compilation_plan();
    let no_param_query =
        compiled_query("listUsers", "SELECT id FROM users;").with_source_path("sql/static.sql");
    let param_query = core::CompiledQuery::new(
        core::QueryId::new("findUserByEmail".to_owned()),
        "SELECT id FROM users WHERE email = ?;".to_owned(),
        core::Cardinality::Many,
        vec![core::InputField::new(
            "email".to_owned(),
            core::CoreType::String,
            false,
        )],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![param("email", core::CoreType::String, false)])
    .with_source_path("sql/static.sql");
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment("SELECT id FROM users WHERE 1 = 1", Vec::new()),
            sql_segment(";", Vec::new()),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![slot_branch("activeOnly", " AND active = 1", Vec::new())],
        )],
    );
    let slot_query = compiled_query("searchUsers", "SELECT id FROM users WHERE 1 = 1;")
        .with_dynamic_body(dynamic_body)
        .with_source_path("sql/dynamic.sql");

    let builders = vec![
        query_builder(no_param_query),
        query_builder(param_query),
        query_builder(slot_query),
    ];
    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("generator should preserve each file's generated surface independently");

    let static_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/static.ts"),
    );
    let dynamic_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/dynamic.ts"),
    );

    assert!(!static_contents.contains("type SqlParam = unknown;"));
    assert!(!static_contents.contains("sqlParts"));
    assert!(!static_contents.contains("readonly SqlParam[]"));
    assert!(static_contents.contains("export type listUsers_Input = Record<string, never>;"));
    assert!(static_contents.contains(
        "export function listUsers(\n  _input: listUsers_Input = {},\n): { sql: string; params: readonly [] }"
    ));
    assert!(static_contents.contains(r#"sql: "SELECT id FROM users;","#));
    assert!(static_contents.contains("params: [] as const,"));
    assert!(static_contents.contains(
        "export function findUserByEmail(\n  input: findUserByEmail_Input,\n): { sql: string; params: readonly [string] }"
    ));
    assert!(static_contents.contains(r#"sql: "SELECT id FROM users WHERE email = ?;","#));
    assert!(static_contents.contains("params: [input.email] as const,"));

    assert!(dynamic_contents.contains("type SqlParam = unknown;"));
    assert!(dynamic_contents.contains("sqlParts.join(\"\")"));
}

#[test]
fn generator_maps_nested_sql_paths_under_output_dir() {
    let plan = compilation_plan();
    let query = compiled_query("listAdmins", "SELECT id FROM admins;")
        .with_source_path("sql/admin/users.sql");

    let builders = vec![query_builder(query)];
    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("generator should map SQL source path to TypeScript output path");

    assert_eq!(files.files().len(), 1);
    assert_eq!(
        files.files()[0].path(),
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/admin/users.ts")
    );
    assert!(
        files.files()[0]
            .contents()
            .contains("export function listAdmins(")
    );
}

/// Verifies fragment-only source sets do not produce header-only modules.
#[test]
fn generator_returns_no_files_when_no_queries_are_compiled() {
    let plan = compilation_plan();

    let files = TypeScriptTargetGenerator
        .generate(&plan, &[])
        .expect("fragment-only source sets should not produce generated files");

    assert!(files.files().is_empty());
}

/// Verifies cross-file Fragment SQL is embedded into the owning query module.
#[test]
fn generator_embeds_cross_file_fragment_branches_in_query_source_output() {
    let plan = compilation_plan();
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment("SELECT id FROM users WHERE 1 = 1", Vec::new()),
            sql_segment(";", Vec::new()),
        ],
        vec![core::CompiledSlotOccurrence::new("filter".to_owned())],
        vec![slot_definition(
            "filter",
            vec![slot_branch("activeOnly", "\nAND active = 1", Vec::new())],
        )],
    );
    let query = compiled_query("listUsers", "SELECT id FROM users WHERE 1 = 1;")
        .with_dynamic_body(dynamic_body)
        .with_source_path("sql/users.sql");

    let builders = vec![query_builder(query)];
    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("query output should embed selected fragment SQL branches");

    assert_eq!(files.files().len(), 1);
    assert_eq!(
        files.files()[0].path(),
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts")
    );
    let users_contents = files.files()[0].contents();
    assert!(users_contents.contains("type SqlParam = unknown;"));
    assert!(users_contents.contains("switch (_input.filter?.$fragment)"));
    assert!(users_contents.contains(r#"sqlParts.push("\nAND active = 1");"#));
    assert!(!users_contents.contains("activeOnly_Input"));
}

#[test]
fn generator_generates_param_queries() {
    let plan = compilation_plan();
    let query = core::CompiledQuery::new(
        core::QueryId::new("findUser".to_owned()),
        "SELECT id FROM users WHERE email = ?;".to_owned(),
        core::Cardinality::Many,
        vec![core::InputField::new(
            "email".to_owned(),
            core::CoreType::String,
            false,
        )],
        vec![core::ResultColumn::new(
            "id".to_owned(),
            core::CoreType::Int64,
            false,
        )],
    )
    .with_params(vec![core::ParamBinding::new(
        "email".to_owned(),
        core::CoreType::String,
        false,
    )])
    .with_source_path("sql/users.sql");

    let builders = vec![query_builder(query)];
    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("Param TypeScript generation should emit input and params");

    let users_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
    );
    assert!(users_contents.contains("export type findUser_Input = {\n  email: string;\n};"));
    assert!(users_contents.contains(
        "export function findUser(\n  input: findUser_Input,\n): { sql: string; params: readonly [string] }"
    ));
    assert!(users_contents.contains("params: [input.email] as const"));
}

#[test]
fn generator_combines_queries_from_same_sql_file_into_one_module() {
    let plan = compilation_plan();
    let builders = vec![
        query_builder(
            compiled_query("listUsers", "SELECT id FROM users;").with_source_path("sql/users.sql"),
        ),
        query_builder(
            compiled_query("findLatestUser", "SELECT id FROM users LIMIT 1;")
                .with_source_path("sql/users.sql"),
        ),
        query_builder(
            compiled_query("listRoles", "SELECT id FROM roles;")
                .with_source_path("sql/admin/roles.sql"),
        ),
    ];

    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("generator should group queries by source SQL file");

    assert_eq!(files.files().len(), 2);
    let users_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
    );
    assert!(users_contents.contains("export function listUsers("));
    assert!(users_contents.contains("export function findLatestUser("));
    assert!(!users_contents.contains("export function listRoles("));
}

#[test]
fn generator_rejects_mutation_builders_until_typescript_generation_exists() {
    let plan = compilation_plan();
    let mutation = core::CompiledMutation::new(
        core::MutationId::new("createUser".to_owned()),
        "INSERT INTO users (email) VALUES (?);".to_owned(),
        core::MutationKind::Insert,
        vec![core::InputField::new(
            "email".to_owned(),
            core::CoreType::String,
            false,
        )],
    )
    .with_params(vec![param("email", core::CoreType::String, false)])
    .with_source_path("sql/users.sql");
    let builders = vec![core::CompiledBuilder::Mutation(mutation)];

    let report = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect_err("mutation builders should be rejected until TypeScript generation exists");

    assert_eq!(report.diagnostics().len(), 1);
    let message = report.diagnostics()[0].message();
    assert!(message.contains("compiled mutation `createUser` reached TypeScript generation"));
    assert!(message.contains("TypeScript mutation builder generation is not implemented yet"));
}

fn query_builder(query: core::CompiledQuery) -> core::CompiledBuilder {
    core::CompiledBuilder::Query(query)
}
