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
fn generator_generates_slotless_mutation_builders_without_row_or_output_aliases() {
    let plan = compilation_plan();
    let mutation = core::CompiledMutation::new(
        core::MutationId::new("createUser".to_owned()),
        "INSERT INTO users (email, name) VALUES (?, ?);".to_owned(),
        core::MutationKind::Insert,
        vec![
            core::InputField::new("email".to_owned(), core::CoreType::String, false),
            core::InputField::new("name".to_owned(), core::CoreType::String, false),
        ],
    )
    .with_params(vec![
        param("email", core::CoreType::String, false),
        param("name", core::CoreType::String, false),
    ])
    .with_source_path("sql/users.sql");
    let builders = vec![core::CompiledBuilder::Mutation(mutation)];

    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("mutation builders should generate TypeScript SQL builders");

    let users_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
    );
    assert!(
        users_contents
            .contains("export type createUser_Input = {\n  email: string;\n  name: string;\n};")
    );
    assert!(users_contents.contains(
        "export function createUser(\n  input: createUser_Input,\n): { sql: string; params: readonly [string, string] }"
    ));
    assert!(users_contents.contains(r#"sql: "INSERT INTO users (email, name) VALUES (?, ?);","#));
    assert!(users_contents.contains("params: [input.email, input.name] as const,"));
    assert!(!users_contents.contains("createUser_Row"));
    assert!(!users_contents.contains("createUser_Output"));
}

#[test]
fn generator_generates_slot_mutation_builders_with_runtime_branches() {
    let plan = compilation_plan();
    let dynamic_body = core::CompiledDynamicQuery::new(
        vec![
            sql_segment(
                "UPDATE users AS u SET name = ?",
                vec![param("name", core::CoreType::String, false)],
            ),
            sql_segment(
                " WHERE u.id = ?;",
                vec![param("id", core::CoreType::Int64, false)],
            ),
        ],
        vec![core::CompiledSlotOccurrence::new("assignment".to_owned())],
        vec![slot_definition(
            "assignment",
            vec![slot_branch(
                "touchUpdatedAt",
                ", updated_at = ?",
                vec![param("updatedAt", core::CoreType::DateTime, false)],
            )],
        )],
    );
    let mutation = core::CompiledMutation::new(
        core::MutationId::new("renameUser".to_owned()),
        "UPDATE users AS u SET name = ? WHERE u.id = ?;".to_owned(),
        core::MutationKind::Update,
        vec![
            core::InputField::new("name".to_owned(), core::CoreType::String, false),
            core::InputField::new("id".to_owned(), core::CoreType::Int64, false),
        ],
    )
    .with_params(vec![
        param("name", core::CoreType::String, false),
        param("id", core::CoreType::Int64, false),
    ])
    .with_dynamic_body(dynamic_body)
    .with_source_path("sql/users.sql");
    let builders = vec![core::CompiledBuilder::Mutation(mutation)];

    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("Slot mutation builders should generate TypeScript SQL builders");

    let users_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
    );
    assert!(users_contents.contains("type SqlParam = unknown;"));
    assert!(users_contents.contains(
        r#"export type renameUser_Input = {
  name: string;
  assignment?: {
    $fragment: "touchUpdatedAt";
    updatedAt: string;
  };
  id: string;
};"#
    ));
    assert!(users_contents.contains(
        "export function renameUser(\n  input: renameUser_Input,\n): { sql: string; params: readonly SqlParam[] }"
    ));
    assert!(users_contents.contains(
        r#"  sqlParts.push("UPDATE users AS u SET name = ?");
  params.push(input.name);
  switch (input.assignment?.$fragment) {
    case "touchUpdatedAt":
      sqlParts.push(", updated_at = ?");
      params.push(input.assignment.updatedAt);
      break;
  }
  sqlParts.push(" WHERE u.id = ?;");
  params.push(input.id);"#
    ));
    assert!(users_contents.contains("sql: sqlParts.join(\"\"),"));
    assert!(!users_contents.contains("renameUser_Row"));
    assert!(!users_contents.contains("renameUser_Output"));
}

#[test]
fn generator_preserves_mixed_query_and_mutation_source_order_in_one_module() {
    let plan = compilation_plan();
    let list_users =
        compiled_query("listUsers", "SELECT id FROM users;").with_source_path("sql/users.sql");
    let create_user = core::CompiledMutation::new(
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
    let find_user = compiled_query("findUser", "SELECT id FROM users LIMIT 1;")
        .with_source_path("sql/users.sql");
    let builders = vec![
        core::CompiledBuilder::Query(list_users),
        core::CompiledBuilder::Mutation(create_user),
        core::CompiledBuilder::Query(find_user),
    ];

    let files = TypeScriptTargetGenerator
        .generate(&plan, &builders)
        .expect("mixed query and mutation builders should generate in source order");

    let users_contents = file_contents(
        &files,
        Path::new("/tmp/sqlay-project/src/generated/sqlay/sql/users.ts"),
    );
    let list_index = users_contents
        .find("export function listUsers(")
        .expect("listUsers should be generated");
    let create_index = users_contents
        .find("export function createUser(")
        .expect("createUser should be generated");
    let find_index = users_contents
        .find("export function findUser(")
        .expect("findUser should be generated");
    assert!(list_index < create_index);
    assert!(create_index < find_index);
}

fn query_builder(query: core::CompiledQuery) -> core::CompiledBuilder {
    core::CompiledBuilder::Query(query)
}
