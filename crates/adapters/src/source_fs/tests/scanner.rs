use super::super::{SqlayBlock, scan_sqlay_blocks};

#[test]
fn returns_empty_scan_when_no_annotation_exists() {
    let source = "SELECT 'plain sql' AS value;\n";
    let scan = scan_sqlay_blocks(source).expect("plain SQL should scan");

    assert!(scan.blocks().is_empty());
    assert_eq!(scan.sql_without_sqlay_blocks(), source);
}

#[test]
fn finds_one_sqlay_block_and_preserves_sql() {
    let source = r"
/* @sqlay
{ type: query, id: listUsers }
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");

    assert_eq!(scan.blocks().len(), 1);
    assert_eq!(
        scan.blocks()[0].payload(),
        "\n{ type: query, id: listUsers }\n"
    );
    assert_eq!(scan.blocks()[0].comment_range().start().line(), 1);
    assert_eq!(scan.blocks()[0].comment_range().start().column(), 1);
    assert_eq!(scan.blocks()[0].payload_range().start().line(), 1);
    assert_eq!(scan.blocks()[0].payload_range().start().column(), 10);
    assert!(!scan.sql_without_sqlay_blocks().contains("@sqlay"));
    assert!(
        scan.sql_without_sqlay_blocks()
            .ends_with("SELECT id FROM users;\n")
    );
}

#[test]
fn scanned_block_equality_ignores_internal_byte_offsets() {
    let source = r"

/* @sqlay
{ type: query, id: listUsers }
*/
SELECT id FROM users;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("annotated SQL should scan");
    let scanned = &scan.blocks()[0];
    let constructed = SqlayBlock::new(
        scanned.payload().to_owned(),
        scanned.comment_range(),
        scanned.payload_range(),
    );

    assert_eq!(*scanned, constructed);
}

#[test]
fn finds_multiple_sqlay_blocks() {
    let source = r"
/* @sqlay
{ id: first }
*/
SELECT 1;
/* @sqlay
{ id: second }
*/
SELECT 2;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("multiple annotations should scan");

    assert_eq!(scan.blocks().len(), 2);
    assert_eq!(scan.blocks()[0].payload(), "\n{ id: first }\n");
    assert_eq!(scan.blocks()[1].payload(), "\n{ id: second }\n");
    assert_eq!(scan.sql_without_sqlay_blocks().matches("SELECT").count(), 2);
}

#[test]
fn ignores_marker_like_text_inside_sql_strings() {
    let source = r#"
SELECT '/* @sqlay { id: nope } */' AS literal, "/* @sqlay */" AS double_quoted;
"#
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("string literal should scan");

    assert!(scan.blocks().is_empty());
    assert_eq!(scan.sql_without_sqlay_blocks(), source);
}

#[test]
fn ignores_marker_like_text_inside_line_comments() {
    let source = r"
-- /* @sqlay { id: nope } */
SELECT 1;
# /* @sqlay */
SELECT 2;
"
    .strip_prefix('\n')
    .expect("raw SQL test source should start with a newline");
    let scan = scan_sqlay_blocks(source).expect("line comments should scan");

    assert!(scan.blocks().is_empty());
    assert_eq!(scan.sql_without_sqlay_blocks(), source);
}

#[test]
fn rejects_unterminated_block_comment() {
    let report = scan_sqlay_blocks(
        r"
SELECT 1;
/* @sqlay
{ id: broken }
"
        .strip_prefix('\n')
        .expect("raw SQL test source should start with a newline"),
    )
    .expect_err("unterminated block comment should be rejected");
    let diagnostic = report
        .diagnostics()
        .first()
        .expect("a diagnostic should be returned");
    let location = diagnostic
        .location()
        .expect("unterminated comment should include location");
    let range = location
        .range()
        .expect("unterminated comment should include source range");

    assert_eq!(diagnostic.message(), "unterminated SQL block comment");
    assert_eq!(range.start().line(), 2);
    assert_eq!(range.start().column(), 1);
}
