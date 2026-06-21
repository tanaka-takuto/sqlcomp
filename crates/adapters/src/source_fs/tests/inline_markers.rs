use super::super::source_units::split_sqlcomp_source_units;
use super::super::split_sqlcomp_query_blocks;
use super::diagnostic_messages;
use sqlcomp_core as core;

#[test]
fn split_query_blocks_keeps_inline_param_markers_inside_query_body() {
    let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: string nullable: true } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let queries = split_sqlcomp_query_blocks(source).expect("inline Param should be accepted");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].metadata().id(), "findUserByEmail");
    assert!(
        queries[0].sql().contains("type: param id: email"),
        "sql: {}",
        queries[0].sql()
    );
    assert!(
        queries[0].sql().contains("type: paramEnd"),
        "sql: {}",
        queries[0].sql()
    );
}

#[test]
fn split_query_blocks_keeps_multiple_query_boundaries_with_inline_params() {
    let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;

/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let queries = split_sqlcomp_query_blocks(source)
        .expect("inline Param should not create extra query boundaries");

    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].metadata().id(), "findUserByEmail");
    assert_eq!(queries[1].metadata().id(), "listUsers");
    assert!(
        !queries[0].sql().contains("id: listUsers"),
        "first query sql: {}",
        queries[0].sql()
    );
    assert_eq!(queries[1].sql(), "\nSELECT id FROM users;\n");
}

#[test]
fn split_query_blocks_replaces_inline_param_ranges_and_records_usages() {
    let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users WHERE email = /* @sqlcomp { type: param id: email valueType: string nullable: true } */ 'test@example.test' /* @sqlcomp { type: paramEnd } */ AND id = /* @sqlcomp { type: param id: userId valueType: int64 } */ 42 /* @sqlcomp { type: paramEnd } */;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
    let queries = split_sqlcomp_query_blocks(source).expect("inline Param should be accepted");

    assert_eq!(queries.len(), 1);
    assert_eq!(
        queries[0].analysis_sql(),
        "\nSELECT id FROM users WHERE email = ? AND id = ?;\n"
    );
    assert_eq!(queries[0].param_usages().len(), 2);
    assert_eq!(queries[0].param_usages()[0].id(), "email");
    assert_eq!(
        queries[0].param_usages()[0].value_type_override(),
        Some(core::CoreType::String)
    );
    assert!(queries[0].param_usages()[0].nullable_override());
    assert_eq!(
        queries[0].param_usages()[0].sample_sql(),
        " 'test@example.test' "
    );
    assert_eq!(queries[0].param_usages()[1].id(), "userId");
    assert_eq!(
        queries[0].param_usages()[1].value_type_override(),
        Some(core::CoreType::Int64)
    );
    assert!(!queries[0].param_usages()[1].nullable_override());

    let range = queries[0].param_usages()[0]
        .source_location()
        .range()
        .expect("Param usage should include the source range");
    assert_eq!(range.start().line(), 7);
}

#[test]
fn split_query_blocks_deletes_inline_slot_markers_and_records_usages() {
    let source = r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT u.id FROM users AS u WHERE 1 = 1/* @sqlcomp { type: slot id: filter targets: [activeOnly, byEmail] } */;
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
    let queries = split_sqlcomp_query_blocks(source).expect("inline Slot should be accepted");

    assert_eq!(queries.len(), 1);
    assert_eq!(
        queries[0].analysis_sql(),
        "\nSELECT u.id FROM users AS u WHERE 1 = 1;\n"
    );
    assert!(!queries[0].analysis_sql().contains("@sqlcomp"));
    assert_eq!(queries[0].slot_usages().len(), 1);
    assert_eq!(queries[0].slot_usages()[0].id(), "filter");
    assert_eq!(
        queries[0].slot_usages()[0].targets(),
        ["activeOnly", "byEmail"]
    );
    assert_eq!(
        &queries[0].analysis_sql()[..queries[0].slot_usages()[0].insertion_index()],
        "\nSELECT u.id FROM users AS u WHERE 1 = 1"
    );
    assert_eq!(
        &queries[0].analysis_sql()[queries[0].slot_usages()[0].insertion_index()..],
        ";\n"
    );
}

#[test]
fn rejects_invalid_inline_slot_metadata() {
    for (source, expected_message) in [
        (
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: 1bad targets: [activeOnly] } */;
",
            "invalid Slot id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: filter } */;
",
            "missing required `slot` metadata field `targets`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: filter targets: [] } */;
",
            "`slot` metadata field `targets` must contain at least one value",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: filter targets: activeOnly } */;
",
            "`slot` metadata field `targets` must be a string array",
        ),
        (
            r#"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: filter targets: ["bad-id"] } */;
"#,
            "invalid Slot target `bad-id`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            r#"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: filter targets: [""] } */;
"#,
            "invalid Slot target ``; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users/* @sqlcomp { type: slot id: filter targets: [activeOnly] required: true } */;
",
            "unknown `slot` metadata field `required`; supported fields are `type`, `id`, and `targets`",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlcomp_query_blocks(source).expect_err("invalid Slot metadata rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}

#[test]
fn rejects_unsupported_inline_slot_placements() {
    for (source, expected_message) in [
        (
            r"
/* @sqlcomp { type: slot id: filter targets: [activeOnly] } */
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
",
            "`slot` markers must appear inside a query body; top-level Slot markers are not supported",
        ),
        (
            r"
/* @sqlcomp
{
  type: fragment
  id: activeOnly
}
*/
/* @sqlcomp { type: slot id: filter targets: [byEmail] } */
AND u.active = 1
",
            "slot markers inside fragments are not supported yet; define slots in query bodies for the initial Slot/Fragment release",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: string } */
  /* @sqlcomp { type: slot id: filter targets: [activeOnly] } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
            "Slot markers are not supported inside Param ranges",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlcomp_source_units(source).expect_err("invalid Slot placement rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}

#[test]
fn split_query_blocks_rejects_raw_or_sample_placeholders() {
    for (source, expected_message) in [
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users WHERE email = ?;
",
            "raw `?` placeholders are not supported in source SQL; use paired `@sqlcomp` Param markers around a sample expression, such as `/* @sqlcomp { type: param id: value } */ 1 /* @sqlcomp { type: paramEnd } */`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users WHERE email = /* @sqlcomp { type: param id: email } */ ? /* @sqlcomp { type: paramEnd } */;
",
            "`?` placeholders are not allowed inside Param sample expressions",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlcomp_query_blocks(source).expect_err("placeholder should be rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
        assert!(
            report.diagnostics()[0].location().is_some(),
            "diagnostic should point to the SQL source"
        );
    }
}

#[test]
fn rejects_invalid_param_ids_at_param_marker_location() {
    let source = r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: 1bad } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let report =
        split_sqlcomp_query_blocks(source).expect_err("invalid Param id should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");
    let range = diagnostic
        .location()
        .and_then(core::SourceLocation::range)
        .expect("Param diagnostic should include source range");

    assert_eq!(
        diagnostic.message(),
        "invalid Param id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`"
    );
    assert_eq!(range.start().line(), 8);
}

#[test]
fn rejects_invalid_inline_param_metadata() {
    for (source, expected_message) in [
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email extra: true } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
            "unknown `param` metadata field `extra`; supported fields are `type`, `id`, `valueType`, and `nullable`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd id: email } */;
",
            "unknown `paramEnd` metadata field `id`; supported fields are `type`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test'
  /* @sqlcomp { type: param_end } */;
",
            "unsupported `@sqlcomp` annotation type `param_end`; use `paramEnd` for Param end markers",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: banana } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
            "unsupported Param valueType `banana`; supported values are `bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: unknown } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
            "unsupported Param valueType `unknown`; supported values are `bool`, `int32`, `int64`, `float64`, `decimal`, `string`, `bytes`, `date`, `time`, `datetime`, and `json`",
        ),
        (
            r#"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email valueType: "string | null" } */
  'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
"#,
            "unsupported Param valueType `string | null`; use `valueType: string` with `nullable: true` for nullable string inputs; optional input properties are not supported",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlcomp_query_blocks(source).expect_err("invalid Param metadata rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}

#[test]
fn rejects_unpaired_or_nested_inline_param_markers() {
    for (source, expected_message) in [
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  'test@example.test';
",
            "`param` marker is missing a matching `paramEnd` marker",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = 'test@example.test'
  /* @sqlcomp { type: paramEnd } */;
",
            "`paramEnd` marker has no matching `param` marker",
        ),
        (
            r"
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users
WHERE email = /* @sqlcomp { type: param id: email } */
  COALESCE(/* @sqlcomp { type: param id: fallbackEmail } */ 'test@example.test'
  /* @sqlcomp { type: paramEnd } */)
  /* @sqlcomp { type: paramEnd } */;
",
            "nested Param ranges are not supported",
        ),
        (
            r"
/* @sqlcomp { type: param id: email } */
'test@example.test'
/* @sqlcomp { type: paramEnd } */
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users;
",
            "`param` markers must appear inside a query or fragment body; top-level Param markers are not supported",
        ),
        (
            r"
/* @sqlcomp { type: paramEnd } */
/* @sqlcomp
{
  type: query
  id: findUserByEmail
}
*/
SELECT id FROM users;
",
            "`paramEnd` markers must appear inside a query or fragment body; top-level paramEnd markers are not supported",
        ),
        (
            r"
/* @sqlcomp
{
  type: fragment
  id: activeOnly
}
*/
AND tenant_id = /* @sqlcomp { type: param id: tenantId valueType: int64 } */
  1
",
            "`param` marker is missing a matching `paramEnd` marker",
        ),
        (
            r"
/* @sqlcomp
{
  type: fragment
  id: activeOnly
}
*/
AND tenant_id = 1
  /* @sqlcomp { type: paramEnd } */
",
            "`paramEnd` marker has no matching `param` marker",
        ),
        (
            r"
/* @sqlcomp
{
  type: fragment
  id: activeOnly
}
*/
AND tenant_id = /* @sqlcomp { type: param id: tenantId valueType: int64 } */
  COALESCE(/* @sqlcomp { type: param id: fallbackTenantId valueType: int64 } */ 1
  /* @sqlcomp { type: paramEnd } */)
  /* @sqlcomp { type: paramEnd } */
",
            "nested Param ranges are not supported",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report = split_sqlcomp_query_blocks(source)
            .expect_err("invalid Param marker structure should be rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}
