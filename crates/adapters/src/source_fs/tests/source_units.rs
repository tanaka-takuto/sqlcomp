use sqlay_core as core;

use super::super::source_units::split_sqlay_source_units;
use super::super::split_sqlay_query_blocks;
use super::diagnostic_messages;

#[test]
fn splits_one_query_block() {
    let source = r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let queries = split_sqlay_query_blocks(source).expect("query block should split");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].metadata().id(), "listUsers");
    assert_eq!(queries[0].sql(), "\nSELECT id FROM users;\n");
    assert!(!queries[0].sql().contains("@sqlay"));
}

#[test]
fn split_query_blocks_attach_sql_body_source_range() {
    let source = r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let queries = split_sqlay_query_blocks(source).expect("query block should split");
    let location = queries[0]
        .source_location()
        .expect("query should include source location");
    let range = location
        .range()
        .expect("query should include SQL body range");

    assert_eq!(location.path(), None);
    assert_eq!(range.start().line(), 7);
    assert_eq!(range.start().column(), 1);
}

#[test]
fn splits_multiple_query_blocks_in_source_order() {
    let source = r"
/* @sqlay
{
  type: query
  id: firstQuery
}
*/
SELECT 1;
/* @sqlay
{
  type: query
  id: secondQuery
}
*/
SELECT 2;
-- trailing file content
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let queries = split_sqlay_query_blocks(source).expect("query blocks should split");

    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].metadata().id(), "firstQuery");
    assert_eq!(queries[0].sql(), "\nSELECT 1;\n");
    assert_eq!(queries[1].metadata().id(), "secondQuery");
    assert_eq!(queries[1].sql(), "\nSELECT 2;\n-- trailing file content\n");
}

#[test]
fn splits_fragment_source_units_and_query_units_in_source_order() {
    let source = r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
-- ordinary SQL comment stays in the fragment body
AND u.active = 1
/* @sqlay { type: param id: tenantId valueType: int64 } */
42
/* @sqlay { type: paramEnd } */
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT u.id FROM users AS u;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");

    let source_units = split_sqlay_source_units(source).expect("source units should split");

    assert_eq!(source_units.fragments().len(), 1);
    assert_eq!(source_units.fragments()[0].metadata().id(), "activeOnly");
    assert_eq!(
        source_units.fragments()[0].sql(),
        "\n-- ordinary SQL comment stays in the fragment body\nAND u.active = 1\n/* @sqlay { type: param id: tenantId valueType: int64 } */\n42\n/* @sqlay { type: paramEnd } */\n"
    );
    assert_eq!(
        source_units.fragments()[0].analysis_sql(),
        "\n-- ordinary SQL comment stays in the fragment body\nAND u.active = 1\n?\n"
    );
    assert_eq!(source_units.fragments()[0].param_usages().len(), 1);
    assert_eq!(
        source_units.fragments()[0].param_usages()[0].id(),
        "tenantId"
    );
    assert_eq!(
        source_units.fragments()[0].param_usages()[0].value_type_override(),
        Some(core::CoreType::Int64)
    );
    assert_eq!(
        source_units.fragments()[0].param_usages()[0].sample_sql(),
        "\n42\n"
    );
    assert_eq!(source_units.queries().len(), 1);
    assert_eq!(source_units.queries()[0].metadata().id(), "listUsers");
    assert_eq!(
        source_units.queries()[0].sql(),
        "\nSELECT u.id FROM users AS u;\n"
    );
}

#[test]
fn rejects_invalid_fragment_metadata() {
    for (source, expected_message) in [
        (
            r"
/* @sqlay
{
  type: fragment
}
*/
AND u.active = 1
",
            "missing required `fragment` metadata field `id`",
        ),
        (
            r"
/* @sqlay
{
  type: fragment
  id: true
}
*/
AND u.active = 1
",
            "`fragment` metadata field `id` must be a string",
        ),
        (
            r"
/* @sqlay
{
  type: fragment
  id: 1bad
}
*/
AND u.active = 1
",
            "invalid fragment id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            r"
/* @sqlay
{
  type: fragment
  id: activeOnly
  cardinality: many
}
*/
AND u.active = 1
",
            "unknown `fragment` metadata field `cardinality`; supported fields are `type` and `id`",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlay_source_units(source).expect_err("invalid fragment metadata rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}

#[test]
fn rejects_invalid_top_level_annotation_metadata() {
    for (source, expected_message) in [
        (
            r"
/* @sqlay
{
  id: listUsers
}
*/
SELECT id FROM users;
",
            "missing required `@sqlay` metadata field `type`",
        ),
        (
            r"
/* @sqlay
{
  type: mutation
  id: updateUser
}
*/
UPDATE users SET name = 'Ada';
",
            "unsupported `@sqlay` annotation type `mutation`; supported values are `query`, `fragment`, `param`, `paramEnd`, and `slot`",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlay_source_units(source).expect_err("invalid annotation metadata rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}

#[test]
fn rejects_boolean_top_level_annotation_type_metadata_as_unsupported_type() {
    let source = r"
/* @sqlay
{
  type: false
  id: listUsers
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let report =
        split_sqlay_source_units(source).expect_err("unsupported annotation type rejected");

    assert_eq!(
        diagnostic_messages(&report),
        [
            "unsupported `@sqlay` annotation type `false`; supported values are `query`, `fragment`, `param`, `paramEnd`, and `slot`"
        ]
    );
}

#[test]
fn rejects_statement_separators_in_fragment_bodies() {
    let source = r"
/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
AND u.active = 1;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");

    let report = split_sqlay_source_units(source)
        .expect_err("fragment statement separators should be rejected");

    assert_eq!(
        diagnostic_messages(&report),
        ["raw statement separator `;` is not supported in fragment bodies"]
    );
}

#[test]
fn allows_statement_separator_text_inside_fragment_literals_and_comments() {
    let source = r"
/* @sqlay
{
  type: fragment
  id: labelled
}
*/
AND u.label = ';'
AND u.escaped = 'escaped \; separator'
AND u.name = 'Ada''; Lovelace'
-- semicolon in comment ;
/* ordinary block comment ; */
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");

    let source_units = split_sqlay_source_units(source)
        .expect("literal and comment semicolons should not be statement separators");

    assert_eq!(source_units.fragments().len(), 1);
    assert_eq!(
        source_units.fragments()[0].sql(),
        "\nAND u.label = ';'\nAND u.escaped = 'escaped \\; separator'\nAND u.name = 'Ada''; Lovelace'\n-- semicolon in comment ;\n/* ordinary block comment ; */\n"
    );
}

#[test]
fn rejects_raw_or_sample_placeholders_in_fragment_bodies() {
    for (source, expected_message) in [
        (
            r"
/* @sqlay
{
  type: fragment
  id: byEmail
}
*/
AND u.email = ?
",
            "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression, such as `/* @sqlay { type: param id: value } */ 1 /* @sqlay { type: paramEnd } */`",
        ),
        (
            r"
/* @sqlay
{
  type: fragment
  id: byEmail
}
*/
AND u.email = /* @sqlay { type: param id: email valueType: string } */ ? /* @sqlay { type: paramEnd } */
",
            "`?` placeholders are not allowed inside Param sample expressions",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlay_source_units(source).expect_err("fragment placeholders should be rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
        assert!(
            report.diagnostics()[0].location().is_some(),
            "diagnostic should point to the SQL source"
        );
    }
}

#[test]
fn splits_adjacent_query_blocks() {
    let source = r"
/* @sqlay
{
  type: query
  id: firstQuery
}
*/SELECT 1;/* @sqlay
{
  type: query
  id: secondQuery
}
*/SELECT 2;"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline");
    let queries = split_sqlay_query_blocks(source).expect("adjacent queries should split");

    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].metadata().id(), "firstQuery");
    assert_eq!(queries[0].sql(), "SELECT 1;");
    assert_eq!(queries[1].metadata().id(), "secondQuery");
    assert_eq!(queries[1].sql(), "SELECT 2;");
}
