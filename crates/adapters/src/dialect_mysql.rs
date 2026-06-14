//! `MySQL` dialect analysis adapter.

use sqlcomp_app::DialectAnalyzer;
use sqlcomp_core as core;
use sqlparser::ast::{Query, SetExpr, Statement};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

const MULTIPLE_STATEMENTS_MESSAGE: &str = "MVP query blocks must contain exactly one SQL statement";
const NON_SELECT_MESSAGE: &str = "MVP query blocks must contain exactly one SELECT statement";
const MISSING_SEMICOLON_MESSAGE: &str = "MVP query blocks must end with `;`";

/// `MySQL` dialect analyzer.
#[derive(Clone, Copy, Debug, Default)]
pub struct MysqlDialectAnalyzer;

impl DialectAnalyzer for MysqlDialectAnalyzer {
    fn analyze(&self, query: &core::RawQuery) -> core::DiagnosticResult<core::AnalyzedQuery> {
        let dialect = MySqlDialect {};

        if !ends_with_statement_terminator(query.sql())? {
            return Err(single_error(MISSING_SEMICOLON_MESSAGE));
        }

        let statements = Parser::parse_sql(&dialect, query.sql())
            .map_err(|error| single_error(format!("failed to parse MySQL query block: {error}")))?;

        let [statement] = statements.as_slice() else {
            return Err(single_error(MULTIPLE_STATEMENTS_MESSAGE));
        };

        if !is_select_statement(statement) {
            return Err(single_error(NON_SELECT_MESSAGE));
        }

        Ok(core::AnalyzedQuery)
    }
}

fn ends_with_statement_terminator(sql: &str) -> core::DiagnosticResult<bool> {
    let dialect = MySqlDialect {};
    let tokens = Tokenizer::new(&dialect, sql)
        .tokenize()
        .map_err(|error| single_error(format!("failed to parse MySQL query block: {error}")))?;

    Ok(matches!(
        tokens
            .iter()
            .rev()
            .find(|token| !matches!(token, Token::Whitespace(_))),
        Some(Token::SemiColon)
    ))
}

fn is_select_statement(statement: &Statement) -> bool {
    matches!(statement, Statement::Query(query) if is_select_query(query))
}

fn is_select_query(query: &Query) -> bool {
    is_select_set_expr(&query.body)
}

fn is_select_set_expr(expr: &SetExpr) -> bool {
    match expr {
        SetExpr::Select(_) => true,
        SetExpr::Query(query) => is_select_query(query),
        SetExpr::SetOperation { left, right, .. } => {
            is_select_set_expr(left) && is_select_set_expr(right)
        }
        SetExpr::Values(_)
        | SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => false,
    }
}

fn single_error(message: impl Into<String>) -> core::DiagnosticReport {
    core::DiagnosticReport::new(core::Diagnostic::error(message))
}

#[cfg(test)]
mod tests {
    use sqlcomp_app::DialectAnalyzer;
    use sqlcomp_core as core;

    use super::MysqlDialectAnalyzer;

    #[test]
    fn accepts_select_statement_ending_with_semicolon() {
        let query = raw_query("SELECT id, name FROM users;");

        let analysis = MysqlDialectAnalyzer.analyze(&query);

        assert_eq!(analysis, Ok(core::AnalyzedQuery));
    }

    #[test]
    fn accepts_select_statement_with_trailing_sql_comment() {
        let query = raw_query("SELECT id FROM users;\n-- kept with the query block at EOF\n");

        let analysis = MysqlDialectAnalyzer.analyze(&query);

        assert_eq!(analysis, Ok(core::AnalyzedQuery));
    }

    #[test]
    fn rejects_insert_statement() {
        assert_rejected(
            "INSERT INTO users (id, name) VALUES (1, 'Ada');",
            "MVP query blocks must contain exactly one SELECT statement",
        );
    }

    #[test]
    fn rejects_update_statement() {
        assert_rejected(
            "UPDATE users SET name = 'Ada' WHERE id = 1;",
            "MVP query blocks must contain exactly one SELECT statement",
        );
    }

    #[test]
    fn rejects_delete_statement() {
        assert_rejected(
            "DELETE FROM users WHERE id = 1;",
            "MVP query blocks must contain exactly one SELECT statement",
        );
    }

    #[test]
    fn rejects_ddl_statement() {
        assert_rejected(
            "CREATE TABLE users (id BIGINT PRIMARY KEY);",
            "MVP query blocks must contain exactly one SELECT statement",
        );
    }

    #[test]
    fn rejects_call_statement() {
        assert_rejected(
            "CALL refresh_users();",
            "MVP query blocks must contain exactly one SELECT statement",
        );
    }

    #[test]
    fn rejects_multiple_statements() {
        assert_rejected(
            "SELECT id FROM users; SELECT name FROM users;",
            "MVP query blocks must contain exactly one SQL statement",
        );
    }

    #[test]
    fn rejects_missing_semicolon() {
        assert_rejected("SELECT id FROM users", "MVP query blocks must end with `;`");
    }

    #[test]
    fn rejects_parser_failures() {
        assert_rejected_with_prefix("SELECT FROM;", "failed to parse MySQL query block");
    }

    fn assert_rejected(sql: &str, expected_message: &str) {
        let query = raw_query(sql);
        let report = MysqlDialectAnalyzer
            .analyze(&query)
            .expect_err("query should be rejected");

        assert_eq!(diagnostic_messages(&report), expected_message);
    }

    fn assert_rejected_with_prefix(sql: &str, expected_prefix: &str) {
        let query = raw_query(sql);
        let report = MysqlDialectAnalyzer
            .analyze(&query)
            .expect_err("query should be rejected");
        let messages = diagnostic_messages(&report);

        assert!(
            messages.starts_with(expected_prefix),
            "expected `{messages}` to start with `{expected_prefix}`"
        );
    }

    fn raw_query(sql: &str) -> core::RawQuery {
        core::RawQuery::new(
            core::QueryMetadata::new("testQuery".to_owned(), None),
            sql.to_owned(),
        )
    }

    fn diagnostic_messages(report: &core::DiagnosticReport) -> String {
        report
            .diagnostics()
            .iter()
            .map(core::Diagnostic::message)
            .collect::<Vec<_>>()
            .join("\n")
    }
}
