//! TypeScript target generation adapter.

use std::fmt::Write as _;

use sqlcomp_app::TargetGenerator;
use sqlcomp_core as core;

/// TypeScript symbols generated from one compiled query ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuerySymbols {
    function: String,
    input_type: String,
    row_type: String,
    output_type: String,
}

impl QuerySymbols {
    /// Build TypeScript symbol names for a compiled query.
    #[must_use]
    pub fn for_query(query: &core::CompiledQuery) -> Self {
        Self::from_query_id(query.id().as_str())
    }

    /// Build TypeScript symbol names from a validated query ID.
    #[must_use]
    pub fn from_query_id(query_id: &str) -> Self {
        Self {
            function: query_id.to_owned(),
            input_type: format!("{query_id}_Input"),
            row_type: format!("{query_id}_Row"),
            output_type: format!("{query_id}_Output"),
        }
    }

    /// Generated query builder function name.
    #[must_use]
    pub fn function_name(&self) -> &str {
        &self.function
    }

    /// Generated input type alias name.
    #[must_use]
    pub fn input_type_name(&self) -> &str {
        &self.input_type
    }

    /// Generated result row type alias name.
    #[must_use]
    pub fn row_type_name(&self) -> &str {
        &self.row_type
    }

    /// Generated output type alias name.
    #[must_use]
    pub fn output_type_name(&self) -> &str {
        &self.output_type
    }
}

/// Render text as a TypeScript double-quoted string literal.
#[must_use]
pub fn typescript_string_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');

    for ch in value.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\u{0008}' => literal.push_str("\\b"),
            '\u{000c}' => literal.push_str("\\f"),
            '\u{2028}' => literal.push_str("\\u2028"),
            '\u{2029}' => literal.push_str("\\u2029"),
            control if control.is_control() => {
                let code_point = u32::from(control);
                write!(&mut literal, "\\u{code_point:04X}").expect("writing to String cannot fail");
            }
            other => literal.push(other),
        }
    }

    literal.push('"');
    literal
}

/// Render a generated query builder `sql` property.
#[must_use]
pub fn render_sql_property(query: &core::CompiledQuery) -> String {
    format!("    sql: {},", typescript_string_literal(query.sql()))
}

/// Dummy TypeScript target generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScriptTargetGenerator;

impl TargetGenerator for TypeScriptTargetGenerator {
    fn generate(
        &self,
        _queries: &[core::CompiledQuery],
    ) -> core::DiagnosticResult<core::GeneratedFiles> {
        Ok(core::GeneratedFiles::new(Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::{QuerySymbols, render_sql_property, typescript_string_literal};
    use sqlcomp_core as core;

    #[test]
    fn query_symbols_use_id_exactly_with_fixed_suffixes() {
        for query_id in ["listUsers", "list_users", "_findUser2", "HTTPStatus200"] {
            let symbols = QuerySymbols::from_query_id(query_id);

            assert_eq!(symbols.function_name(), query_id);
            assert_eq!(symbols.input_type_name(), format!("{query_id}_Input"));
            assert_eq!(symbols.row_type_name(), format!("{query_id}_Row"));
            assert_eq!(symbols.output_type_name(), format!("{query_id}_Output"));
        }
    }

    #[test]
    fn query_symbols_are_derived_from_compiled_query_id_without_transformation() {
        let query = core::CompiledQuery::new(
            core::QueryId::new("list_users".to_owned()),
            "SELECT id FROM users;".to_owned(),
            core::Cardinality::Many,
            Vec::new(),
            Vec::new(),
        );

        let symbols = QuerySymbols::for_query(&query);

        assert_eq!(symbols.function_name(), "list_users");
        assert_eq!(symbols.input_type_name(), "list_users_Input");
        assert_eq!(symbols.row_type_name(), "list_users_Row");
        assert_eq!(symbols.output_type_name(), "list_users_Output");
    }

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
}
