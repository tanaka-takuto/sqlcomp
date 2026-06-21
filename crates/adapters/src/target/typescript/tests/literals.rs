use sqlay_core as core;

use super::super::{render_sql_property, typescript_string_literal};

#[test]
fn sql_literal_uses_double_quotes_for_template_literal_hazards() {
    let sql = "SELECT `id`, '${literal}' FROM `users` WHERE note = '${not_param}';";

    assert_eq!(
        typescript_string_literal(sql),
        r#""SELECT `id`, '${literal}' FROM `users` WHERE note = '${not_param}';""#
    );
}

#[test]
fn sql_literal_escapes_quotes_backslashes_and_line_breaks() {
    let sql = "SELECT \"quoted\", 'single', C:\\tmp\\users\nFROM users\r\nWHERE tab = '\t';";

    assert_eq!(
        typescript_string_literal(sql),
        r#""SELECT \"quoted\", 'single', C:\\tmp\\users\nFROM users\r\nWHERE tab = '\t';""#
    );
}

#[test]
fn sql_literal_escapes_javascript_line_separators_and_other_controls() {
    let sql = "SELECT '\u{0001}\u{2028}\u{2029}';";

    assert_eq!(
        typescript_string_literal(sql),
        r#""SELECT '\u0001\u2028\u2029';""#
    );
}

#[test]
fn rendered_sql_property_uses_safe_literal() {
    let query = core::CompiledQuery::new(
        core::QueryId::new("findNotes".to_owned()),
        "SELECT `body`, '${not_param}'\nFROM notes;".to_owned(),
        core::Cardinality::Many,
        Vec::new(),
        Vec::new(),
    );

    assert_eq!(
        render_sql_property(&query),
        r#"    sql: "SELECT `body`, '${not_param}'\nFROM notes;","#
    );
}
