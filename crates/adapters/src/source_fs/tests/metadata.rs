use sqlay_core as core;

use super::super::{parse_sqlay_query_metadata, scan_sqlay_blocks};

#[test]
fn parses_query_metadata_from_hjson_payload() {
    let source = r"
/* @sqlay
{
  type: query
  id: listUsers
  cardinality: one
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let metadata =
        parse_sqlay_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

    assert_eq!(metadata.id(), "listUsers");
    assert_eq!(metadata.cardinality(), Some(core::Cardinality::One));
}

#[test]
fn parses_query_metadata_without_optional_cardinality() {
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
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let metadata =
        parse_sqlay_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

    assert_eq!(metadata.id(), "listUsers");
    assert_eq!(metadata.cardinality(), None);
}

#[test]
fn parses_query_metadata_id_with_nullable_prefix() {
    let source = r"
/* @sqlay
{
  type: query
  id: nullableParamAttempt
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let metadata =
        parse_sqlay_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

    assert_eq!(metadata.id(), "nullableParamAttempt");
    assert_eq!(metadata.cardinality(), None);
}

#[test]
fn accepts_supported_cardinality_values() {
    for (raw_cardinality, cardinality) in [
        ("one", core::Cardinality::One),
        ("many", core::Cardinality::Many),
    ] {
        let source = format!(
            r"
/* @sqlay
{{
  type: query
  id: listUsers
  cardinality: {raw_cardinality}
}}
*/
SELECT id FROM users;
"
        );
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
        let metadata =
            parse_sqlay_query_metadata(&scan.blocks()[0]).expect("query metadata should parse");

        assert_eq!(metadata.cardinality(), Some(cardinality));
    }
}

#[test]
fn rejects_missing_required_query_metadata_fields() {
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
  type: query
}
*/
SELECT id FROM users;
",
            "missing required `@sqlay` metadata field `id`",
        ),
    ] {
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
        let report = parse_sqlay_query_metadata(&scan.blocks()[0])
            .expect_err("missing required metadata should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");

        assert_eq!(diagnostic.message(), expected_message);
        assert!(diagnostic.location().is_some());
    }
}

#[test]
fn rejects_exec_cardinality_reserved_for_future_statement_support() {
    let source = r"
/* @sqlay
{
  type: query
  id: listUsers
  cardinality: exec
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let report = parse_sqlay_query_metadata(&scan.blocks()[0])
        .expect_err("exec cardinality should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");

    assert_eq!(
        diagnostic.message(),
        "`cardinality: exec` is reserved for future non-SELECT support and is not currently supported"
    );
    assert!(diagnostic.location().is_some());
}

#[test]
fn rejects_unsupported_cardinality_values() {
    let source = r"
/* @sqlay
{
  type: query
  id: listUsers
  cardinality: maybe
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let report = parse_sqlay_query_metadata(&scan.blocks()[0])
        .expect_err("unsupported cardinality should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");

    assert_eq!(
        diagnostic.message(),
        "unsupported query cardinality `maybe`; supported values are `one` and `many`"
    );
    assert!(diagnostic.location().is_some());
}

#[test]
fn rejects_malformed_hjson_metadata() {
    let source = r"
/* @sqlay
{
  type query
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let report = parse_sqlay_query_metadata(&scan.blocks()[0])
        .expect_err("malformed Hjson should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");

    assert!(
        diagnostic
            .message()
            .starts_with("failed to parse `@sqlay` metadata as Hjson:")
    );
    assert!(diagnostic.location().is_some());
}

#[test]
fn rejects_unsupported_annotation_types() {
    let source = r"
/* @sqlay
{
  type: param
  id: userId
}
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let report = parse_sqlay_query_metadata(&scan.blocks()[0])
        .expect_err("unsupported annotation type should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");

    assert_eq!(
        diagnostic.message(),
        "unsupported `@sqlay` annotation type `param`; expected `query` metadata"
    );
    assert!(diagnostic.location().is_some());
}

#[test]
fn rejects_invalid_query_ids() {
    for id in ["1bad", "list-users", "\"\""] {
        let source = format!(
            r"
/* @sqlay
{{
  type: query
  id: {id}
}}
*/
SELECT 1;
"
        );
        let source = source
            .strip_prefix('\n')
            .expect("raw SQL test source should start with a newline");
        let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
        let report = parse_sqlay_query_metadata(&scan.blocks()[0])
            .expect_err("invalid query id should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("a diagnostic should be returned");
        let displayed_id = id.trim_matches('"');

        assert_eq!(
            diagnostic.message(),
            format!("invalid query id `{displayed_id}`; must match `^[A-Za-z_][A-Za-z0-9_]*$`")
        );
        assert!(diagnostic.location().is_some());
    }
}
