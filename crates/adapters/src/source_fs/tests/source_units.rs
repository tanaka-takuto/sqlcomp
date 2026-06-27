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
fn splits_mutation_source_units_and_preserves_mixed_source_order() {
    let source = r"
/* @sqlay
{
  type: query
  id: listUsers
}
*/
SELECT u.id FROM users AS u;
/* @sqlay
{
  type: mutation
  id: createUser
}
*/
INSERT INTO users (email)
VALUES (
  /* @sqlay { type: param id: email valueType: string } */
  'ada@example.test'
  /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: slot id: writeMode targets: [upsertName] } */;
/* @sqlay
{
  type: fragment
  id: upsertName
}
*/
ON DUPLICATE KEY UPDATE name = VALUES(name)
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");

    let source_units = split_sqlay_source_units(source).expect("source units should split");

    assert_eq!(source_units.queries().len(), 1);
    assert_eq!(source_units.mutations().len(), 1);
    assert_eq!(source_units.fragments().len(), 1);
    assert_eq!(source_units.source_units().len(), 3);
    assert!(matches!(
        source_units.source_units()[0],
        core::RawSourceUnit::Query(_)
    ));
    assert!(matches!(
        source_units.source_units()[1],
        core::RawSourceUnit::Mutation(_)
    ));
    assert!(matches!(
        source_units.source_units()[2],
        core::RawSourceUnit::Fragment(_)
    ));
    assert_eq!(source_units.source_units()[0].id(), "listUsers");
    assert_eq!(source_units.source_units()[1].id(), "createUser");
    assert_eq!(source_units.source_units()[2].id(), "upsertName");

    let mutation = &source_units.mutations()[0];
    assert_eq!(mutation.metadata().id(), "createUser");
    assert_eq!(
        mutation.analysis_sql(),
        "\nINSERT INTO users (email)\nVALUES (\n  ?\n)\n;\n"
    );
    assert_eq!(mutation.param_usages().len(), 1);
    assert_eq!(mutation.param_usages()[0].id(), "email");
    assert_eq!(
        mutation.param_usages()[0].value_type_override(),
        Some(core::CoreType::String)
    );
    assert_eq!(
        mutation.param_usages()[0].sample_sql(),
        "\n  'ada@example.test'\n  "
    );
    assert_eq!(mutation.slot_usages().len(), 1);
    assert_eq!(mutation.slot_usages()[0].id(), "writeMode");
    assert_eq!(mutation.slot_usages()[0].targets(), ["upsertName"]);
}

#[test]
fn splits_repeat_usages_inside_mutation_and_fragment_source_units() {
    let source = r#"
/* @sqlay
{
  type: mutation
  id: createUsers
}
*/
INSERT INTO users (email)
VALUES /* @sqlay { type: repeat id: rows separator: "," } */ (/* @sqlay { type: param id: email valueType: string } */ 'ada@example.test' /* @sqlay { type: paramEnd } */) /* @sqlay { type: repeatEnd } */;
/* @sqlay
{
  type: fragment
  id: byIds
}
*/
AND u.id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */)
"#
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");

    let source_units = split_sqlay_source_units(source).expect("source units should split");

    assert_eq!(source_units.mutations().len(), 1);
    let mutation = &source_units.mutations()[0];
    assert_eq!(mutation.param_usages(), []);
    assert_eq!(mutation.repeat_usages().len(), 1);
    assert_eq!(mutation.repeat_usages()[0].id(), "rows");
    assert_eq!(mutation.repeat_usages()[0].separator(), ",");
    assert_eq!(mutation.repeat_usages()[0].item_param_usages().len(), 1);
    assert_eq!(
        mutation.repeat_usages()[0].item_param_usages()[0].id(),
        "email"
    );
    assert_eq!(
        &mutation.analysis_sql()
            [mutation.repeat_usages()[0].start_index()..mutation.repeat_usages()[0].end_index()],
        " (?) "
    );

    assert_eq!(source_units.fragments().len(), 1);
    let fragment = &source_units.fragments()[0];
    assert_eq!(fragment.param_usages(), []);
    assert_eq!(fragment.repeat_usages().len(), 1);
    assert_eq!(fragment.repeat_usages()[0].id(), "ids");
    assert_eq!(fragment.repeat_usages()[0].separator(), ",");
    assert_eq!(fragment.repeat_usages()[0].item_param_usages().len(), 1);
    assert_eq!(
        fragment.repeat_usages()[0].item_param_usages()[0].id(),
        "id"
    );
    assert_eq!(
        &fragment.analysis_sql()
            [fragment.repeat_usages()[0].start_index()..fragment.repeat_usages()[0].end_index()],
        " ? "
    );
}

#[test]
fn rejects_invalid_mutation_metadata() {
    for (source, expected_message) in [
        (
            r"
/* @sqlay
{
  type: mutation
}
*/
INSERT INTO users (email) VALUES ('ada@example.test');
",
            "missing required `mutation` metadata field `id`",
        ),
        (
            r"
/* @sqlay
{
  type: mutation
  id: true
}
*/
INSERT INTO users (email) VALUES ('ada@example.test');
",
            "`mutation` metadata field `id` must be a string",
        ),
        (
            r"
/* @sqlay
{
  type: mutation
  id: 1bad
}
*/
INSERT INTO users (email) VALUES ('ada@example.test');
",
            "invalid mutation id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            r"
/* @sqlay
{
  type: mutation
  id: createUser
  cardinality: one
}
*/
INSERT INTO users (email) VALUES ('ada@example.test');
",
            "unknown `mutation` metadata field `cardinality`; supported fields are `type` and `id`",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlay_source_units(source).expect_err("invalid mutation metadata rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
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
  type: procedure
  id: rebuildSearchIndex
}
*/
CALL rebuild_search_index();
",
            "unsupported `@sqlay` annotation type `procedure`; supported values are `query`, `mutation`, `fragment`, `param`, `paramEnd`, `slot`, `repeat`, and `repeatEnd`",
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
            "unsupported `@sqlay` annotation type `false`; supported values are `query`, `mutation`, `fragment`, `param`, `paramEnd`, `slot`, `repeat`, and `repeatEnd`"
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
