use super::super::source_units::split_sqlay_source_units;
use super::super::split_sqlay_query_blocks;
use super::diagnostic_messages;
use sqlay_core as core;

#[test]
fn split_query_blocks_records_repeat_usages_with_item_params() {
    let source = r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT u.id
FROM users AS u
WHERE u.id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* ordinary item comment */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
"#
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let queries = split_sqlay_query_blocks(source).expect("inline Repeat should be accepted");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].metadata().id(), "findUsers");
    assert_eq!(queries[0].param_usages(), []);
    assert_eq!(queries[0].repeat_usages().len(), 1);

    let repeat = &queries[0].repeat_usages()[0];
    assert_eq!(repeat.id(), "ids");
    assert_eq!(repeat.separator(), ",");
    assert_eq!(repeat.item_param_usages().len(), 1);
    assert_eq!(repeat.item_param_usages()[0].id(), "id");
    assert_eq!(
        repeat.item_param_usages()[0].value_type_override(),
        Some(core::CoreType::Int64)
    );
    assert_eq!(
        &queries[0].analysis_sql()[repeat.start_index()..repeat.end_index()],
        " /* ordinary item comment */ ? "
    );
    assert_eq!(
        repeat.item_param_usages()[0].placeholder_index(),
        Some(repeat.start_index() + " /* ordinary item comment */ ".len())
    );

    let range = repeat
        .source_location()
        .range()
        .expect("Repeat usage should include the source range");
    assert_eq!(range.start().line(), 9);
}

#[test]
fn rejects_invalid_inline_repeat_metadata() {
    for (source, expected_message) in [
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
"#,
            "missing required `repeat` metadata field `id`",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: true separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
"#,
            "`repeat` metadata field `id` must be a string",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: 1bad separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
"#,
            "invalid Repeat id `1bad`; must match `^[A-Za-z_][A-Za-z0-9_]*$`",
        ),
        (
            r"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
",
            "missing required `repeat` metadata field `separator`",
        ),
        (
            r"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: true } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
",
            "`repeat` metadata field `separator` must be a string",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," minItems: 1 } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
"#,
            "unknown `repeat` metadata field `minItems`; supported fields are `type`, `id`, and `separator`",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd id: ids } */);
"#,
            "unknown `repeatEnd` metadata field `id`; supported fields are `type`",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlay_query_blocks(source).expect_err("invalid Repeat metadata rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}

#[test]
fn rejects_unsupported_inline_repeat_placements() {
    for (source, expected_message) in [
        (
            r#"
/* @sqlay { type: repeat id: ids separator: "," } */
/* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */
/* @sqlay { type: repeatEnd } */
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users;
"#,
            "`repeat` markers must appear inside a query, mutation, or fragment body; top-level Repeat markers are not supported",
        ),
        (
            r"
/* @sqlay { type: repeatEnd } */
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users;
",
            "`repeatEnd` markers must appear inside a query, mutation, or fragment body; top-level repeatEnd markers are not supported",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */);
"#,
            "`repeat` marker is missing a matching `repeatEnd` marker",
        ),
        (
            r"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */);
",
            "`repeatEnd` marker has no matching `repeat` marker",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* @sqlay { type: repeat id: nestedIds separator: "," } */ /* @sqlay { type: param id: id valueType: int64 } */ 1 /* @sqlay { type: paramEnd } */ /* @sqlay { type: repeatEnd } */ /* @sqlay { type: repeatEnd } */);
"#,
            "nested Repeat ranges are not supported",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users
WHERE email = /* @sqlay { type: param id: email valueType: string } */
  COALESCE(/* @sqlay { type: repeat id: values separator: "," } */ 'ada@example.test' /* @sqlay { type: repeatEnd } */)
  /* @sqlay { type: paramEnd } */;
"#,
            "Repeat markers are not supported inside Param ranges",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," } */ /* @sqlay { type: slot id: filter targets: [activeOnly] } */ 1 /* @sqlay { type: repeatEnd } */);
"#,
            "Slot markers are not supported inside Repeat ranges",
        ),
        (
            r#"
/* @sqlay
{
  type: query
  id: findUsers
}
*/
SELECT id FROM users WHERE id IN (/* @sqlay { type: repeat id: ids separator: "," } */ 1 /* @sqlay { type: repeatEnd } */);
"#,
            "Repeat ranges must contain at least one Param marker",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let report =
            split_sqlay_source_units(source).expect_err("invalid Repeat placement rejected");

        assert_eq!(diagnostic_messages(&report), [expected_message]);
    }
}
