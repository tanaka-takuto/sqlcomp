//! `MySQL` dialect analysis adapter.

use sqlcomp_app::DialectAnalyzer;
use sqlcomp_core as core;
use sqlparser::ast::{Expr, LimitClause, Query, SetExpr, Statement};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

/// `MySQL` dialect analyzer backed by `sqlparser-rs`.
#[derive(Clone, Copy, Debug, Default)]
pub struct MysqlDialectAnalyzer;

impl DialectAnalyzer for MysqlDialectAnalyzer {
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        let dialect = MySqlDialect {};
        let statements = Parser::parse_sql(&dialect, query.sql())
            .map_err(|error| query_error(query, format!("failed to parse MySQL SQL: {error}")))?;

        let [statement] = statements.as_slice() else {
            return Err(query_error(
                query,
                format!(
                    "expected exactly one SQL statement per query block; found {}",
                    statements.len()
                ),
            ));
        };

        let tokens = tokenize_query(query)?;
        if !ends_with_statement_terminator(&tokens) {
            return Err(query_error(query, "query block must end with `;`"));
        }

        reject_unsupported_placeholders(query, &tokens)?;

        let Statement::Query(parsed_query) = statement else {
            return Err(unsupported_statement_error(query, statement));
        };

        if !is_select_query(parsed_query) {
            return Err(unsupported_statement_error(query, statement));
        }

        Ok(core::AnalyzedQuery::new(infer_cardinality(parsed_query)))
    }
}

fn tokenize_query(query: &core::RawQuery) -> core::DiagnosticResult<Vec<Token>> {
    let dialect = MySqlDialect {};
    Tokenizer::new(&dialect, query.sql())
        .tokenize()
        .map_err(|error| query_error(query, format!("failed to parse MySQL SQL: {error}")))
}

fn ends_with_statement_terminator(tokens: &[Token]) -> bool {
    matches!(
        tokens
            .iter()
            .rev()
            .find(|token| !matches!(token, Token::Whitespace(_))),
        Some(Token::SemiColon)
    )
}

fn reject_unsupported_placeholders(
    query: &core::RawQuery,
    tokens: &[Token],
) -> core::DiagnosticResult<()> {
    if tokens
        .iter()
        .any(|token| matches!(token, Token::Placeholder(_)))
    {
        return Err(query_error(
            query,
            "query parameters/placeholders are not supported in the MVP",
        ));
    }

    Ok(())
}

fn infer_cardinality(query: &Query) -> core::Cardinality {
    if query.limit_clause.as_ref().is_some_and(limit_clause_is_one) {
        core::Cardinality::One
    } else {
        core::Cardinality::Many
    }
}

fn limit_clause_is_one(limit_clause: &LimitClause) -> bool {
    match limit_clause {
        LimitClause::LimitOffset {
            limit: Some(limit), ..
        }
        | LimitClause::OffsetCommaLimit { limit, .. } => expression_is_one(limit),
        LimitClause::LimitOffset { limit: None, .. } => false,
    }
}

fn expression_is_one(expression: &Expr) -> bool {
    matches!(expression, Expr::Value(value) if value.to_string() == "1")
}

fn is_select_query(query: &Query) -> bool {
    is_select_set_expression(query.body.as_ref())
}

fn is_select_set_expression(expression: &SetExpr) -> bool {
    match expression {
        SetExpr::Select(_) => true,
        SetExpr::Query(query) => is_select_query(query),
        SetExpr::SetOperation { left, right, .. } => {
            is_select_set_expression(left) && is_select_set_expression(right)
        }
        SetExpr::Values(_)
        | SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => false,
    }
}

fn unsupported_statement_error(
    query: &core::RawQuery,
    statement: &Statement,
) -> core::DiagnosticReport {
    query_error(
        query,
        format!(
            "unsupported SQL statement `{}`; MVP only supports SELECT queries",
            statement_keyword(statement)
        ),
    )
}

fn statement_keyword(statement: &Statement) -> String {
    statement
        .to_string()
        .split_whitespace()
        .next()
        .unwrap_or("SQL")
        .trim_end_matches(';')
        .to_ascii_uppercase()
}

fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

#[cfg(test)]
mod tests {
    use super::MysqlDialectAnalyzer;
    use sqlcomp_app::DialectAnalyzer;
    use sqlcomp_core as core;

    #[test]
    fn accepts_simple_select_and_infers_many() {
        let analysis =
            analyze_sql("SELECT id, name FROM users;").expect("simple SELECT should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn accepts_select_with_trailing_sql_comment() {
        let analysis = analyze_sql("SELECT id FROM users;\n-- kept with the query block at EOF\n")
            .expect("trailing comments after the terminator should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn accepts_mysql_dialect_syntax() {
        let analysis = analyze_sql("SELECT `id` FROM `users` LIMIT 10, 20;")
            .expect("MySQL-specific SELECT should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn accepts_question_marks_inside_string_literals() {
        let analysis = analyze_sql("SELECT '?' AS literal_text;")
            .expect("question marks inside SQL literals should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn accepts_select_set_operations() {
        let analysis = analyze_sql("SELECT id FROM users UNION SELECT id FROM archived_users;")
            .expect("SELECT set operations should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn infers_one_for_top_level_limit_one() {
        let analysis = analyze_sql("SELECT id FROM users ORDER BY id DESC LIMIT 1;")
            .expect("LIMIT 1 SELECT should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::One);
    }

    #[test]
    fn infers_one_for_mysql_offset_comma_limit_one() {
        let analysis = analyze_sql("SELECT id FROM users ORDER BY id DESC LIMIT 20, 1;")
            .expect("MySQL offset-comma LIMIT 1 SELECT should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::One);
    }

    #[test]
    fn ignores_limit_one_inside_subquery_for_cardinality() {
        let analysis = analyze_sql(
            "SELECT (SELECT id FROM users ORDER BY id DESC LIMIT 1) AS latest_id FROM accounts;",
        )
        .expect("subquery LIMIT 1 should be accepted");

        assert_eq!(analysis.cardinality(), core::Cardinality::Many);
    }

    #[test]
    fn rejects_sql_that_mysql_parser_cannot_parse() {
        let location = core::SourceLocation::at_position(
            "sql/users.sql",
            core::SourcePosition::one_based(12, 3).expect("test position should be valid"),
        );
        let query = raw_query("SELECT FROM;").with_source_location(location.clone());
        let report = MysqlDialectAnalyzer
            .analyze(&query)
            .expect_err("invalid SQL should be rejected");
        let diagnostic = report
            .diagnostics()
            .first()
            .expect("parser failure should produce a diagnostic");

        assert!(
            diagnostic
                .message()
                .starts_with("failed to parse MySQL SQL:"),
            "message: {}",
            diagnostic.message()
        );
        assert_eq!(diagnostic.location(), Some(&location));
    }

    #[test]
    fn rejects_multiple_sql_statements() {
        let report = analyze_sql("SELECT 1; SELECT 2;")
            .expect_err("multi-statement query blocks should be rejected");

        assert_eq!(
            report.diagnostics()[0].message(),
            "expected exactly one SQL statement per query block; found 2"
        );
    }

    #[test]
    fn rejects_empty_query_block() {
        let report = analyze_sql("").expect_err("empty query blocks should be rejected");

        assert_eq!(
            report.diagnostics()[0].message(),
            "expected exactly one SQL statement per query block; found 0"
        );
    }

    #[test]
    fn rejects_missing_semicolon() {
        let report =
            analyze_sql("SELECT id FROM users").expect_err("missing semicolon should be rejected");

        assert_eq!(
            report.diagnostics()[0].message(),
            "query block must end with `;`"
        );
    }

    #[test]
    fn rejects_positional_placeholders_while_mvp_params_are_unsupported() {
        let report = analyze_sql("SELECT id FROM users WHERE email = ?;")
            .expect_err("MVP query parameters should be rejected");

        assert_eq!(
            report.diagnostics()[0].message(),
            "query parameters/placeholders are not supported in the MVP"
        );
    }

    #[test]
    fn rejects_insert_statement() {
        assert_unsupported_statement("INSERT INTO users (id, name) VALUES (1, 'Ada');", "INSERT");
    }

    #[test]
    fn rejects_update_statement() {
        assert_unsupported_statement("UPDATE users SET name = 'Ada';", "UPDATE");
    }

    #[test]
    fn rejects_delete_statement() {
        assert_unsupported_statement("DELETE FROM users WHERE id = 1;", "DELETE");
    }

    #[test]
    fn rejects_ddl_statement() {
        assert_unsupported_statement("CREATE TABLE users (id BIGINT PRIMARY KEY);", "CREATE");
    }

    #[test]
    fn rejects_call_statement() {
        assert_unsupported_statement("CALL refresh_users();", "CALL");
    }

    #[test]
    fn rejects_values_query_expression() {
        let report =
            analyze_sql("VALUES ROW(1);").expect_err("VALUES query expressions are not SELECT");

        assert_eq!(
            report.diagnostics()[0].message(),
            "unsupported SQL statement `VALUES`; MVP only supports SELECT queries"
        );
    }

    fn assert_unsupported_statement(sql: &str, statement: &str) {
        let report = analyze_sql(sql).expect_err("unsupported statement should be rejected");

        assert_eq!(
            report.diagnostics()[0].message(),
            format!("unsupported SQL statement `{statement}`; MVP only supports SELECT queries")
        );
    }

    fn analyze_sql(sql: &str) -> core::DiagnosticResult<core::AnalyzedQuery> {
        MysqlDialectAnalyzer.analyze(&raw_query(sql))
    }

    fn raw_query(sql: &str) -> core::RawQuery {
        core::RawQuery::new(
            core::QueryMetadata::new("testQuery".to_owned(), None),
            sql.to_owned(),
        )
    }
}
