use sqlay_app::MutationAnalyzer;
use sqlay_core as core;
use sqlparser::ast::{
    Delete, FromTable, Insert, OnInsert, Query, SetExpr, Statement, TableFactor, TableObject,
    TableWithJoins, Update,
};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

use super::{
    MysqlDialectAnalyzer, RAW_PLACEHOLDER_GUIDANCE, ends_with_statement_terminator,
    statement_keyword,
};
use crate::diagnostics::{mutation_error, mutation_param_usage_error};

impl MysqlDialectAnalyzer {
    /// Analyze one raw mutation under the supported `MySQL` mutation subset.
    ///
    /// # Errors
    ///
    /// Returns diagnostics when SQL is invalid for `MySQL` or outside ADR 0010's
    /// initial mutation statement scope.
    pub fn analyze_mutation(
        &self,
        mutation: &core::RawMutation,
    ) -> core::DiagnosticResult<core::AnalyzedMutation> {
        let dialect = MySqlDialect {};
        let statements = Parser::parse_sql(&dialect, mutation.analysis_sql()).map_err(|error| {
            mutation_error(mutation, format!("failed to parse MySQL SQL: {error}"))
        })?;

        let [statement] = statements.as_slice() else {
            return Err(mutation_error(
                mutation,
                format!(
                    "expected exactly one SQL statement per mutation block; found {}",
                    statements.len()
                ),
            ));
        };

        let tokens = tokenize_mutation(mutation)?;
        if !ends_with_statement_terminator(&tokens) {
            return Err(mutation_error(mutation, "mutation block must end with `;`"));
        }

        reject_unsupported_mutation_placeholders(mutation, &tokens)?;
        validate_mutation_param_sample_expressions(mutation)?;

        let kind = analyze_mutation_statement(mutation, statement)?;

        Ok(core::AnalyzedMutation::new(kind))
    }
}

impl MutationAnalyzer for MysqlDialectAnalyzer {
    fn analyze_mutation(
        &self,
        mutation: &core::RawMutation,
    ) -> core::DiagnosticResult<core::AnalyzedMutation> {
        Self::analyze_mutation(self, mutation)
    }
}

fn analyze_mutation_statement(
    mutation: &core::RawMutation,
    statement: &Statement,
) -> core::DiagnosticResult<core::MutationKind> {
    match statement {
        Statement::Insert(insert) if insert.replace_into => {
            validate_insert_or_replace(mutation, insert, core::MutationKind::Replace)?;
            Ok(core::MutationKind::Replace)
        }
        Statement::Insert(insert) => {
            validate_insert_or_replace(mutation, insert, core::MutationKind::Insert)?;
            Ok(core::MutationKind::Insert)
        }
        Statement::Update(update) => {
            validate_update(mutation, update)?;
            Ok(core::MutationKind::Update)
        }
        Statement::Delete(delete) => {
            validate_delete(mutation, delete)?;
            Ok(core::MutationKind::Delete)
        }
        _ => Err(mutation_error(
            mutation,
            format!(
                "unsupported mutation SQL statement `{}`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
                statement_keyword(statement)
            ),
        )),
    }
}

fn validate_insert_or_replace(
    mutation: &core::RawMutation,
    insert: &Insert,
    kind: core::MutationKind,
) -> core::DiagnosticResult<()> {
    if !matches!(insert.table, TableObject::TableName(_)) {
        return Err(unsupported_insert_or_replace_form(mutation, kind));
    }
    if insert.returning.is_some()
        || insert.output.is_some()
        || insert.overwrite
        || insert.partitioned.is_some()
        || !insert.after_columns.is_empty()
        || insert.has_table_keyword
        || insert.settings.is_some()
        || insert.format_clause.is_some()
        || insert.multi_table_insert_type.is_some()
        || !insert.multi_table_into_clauses.is_empty()
        || !insert.multi_table_when_clauses.is_empty()
        || insert.multi_table_else_clause.is_some()
    {
        return Err(unsupported_insert_or_replace_form(mutation, kind));
    }
    if matches!(insert.on, Some(OnInsert::OnConflict(_))) {
        return Err(unsupported_insert_or_replace_form(mutation, kind));
    }
    if kind == core::MutationKind::Replace && insert.on.is_some() {
        return Err(unsupported_insert_or_replace_form(mutation, kind));
    }

    let has_set_assignments = !insert.assignments.is_empty();
    let has_values_source = insert
        .source
        .as_ref()
        .is_some_and(|query| query_is_values(query));
    if has_set_assignments && insert.source.is_none() {
        return Ok(());
    }
    if !has_set_assignments && has_values_source {
        return Ok(());
    }
    if insert.source.is_some() {
        return Err(insert_or_replace_select_error(mutation, kind));
    }

    Err(unsupported_insert_or_replace_form(mutation, kind))
}

fn validate_update(mutation: &core::RawMutation, update: &Update) -> core::DiagnosticResult<()> {
    if update.selection.is_none() {
        return Err(mutation_error(
            mutation,
            "UPDATE mutation requires a WHERE clause",
        ));
    }
    if !is_single_table_with_optional_alias(&update.table)
        || update.from.is_some()
        || update.returning.is_some()
        || update.output.is_some()
    {
        return Err(mutation_error(
            mutation,
            "unsupported multi-table UPDATE; initial mutation support only accepts single-table UPDATE",
        ));
    }

    Ok(())
}

fn validate_delete(mutation: &core::RawMutation, delete: &Delete) -> core::DiagnosticResult<()> {
    if delete.selection.is_none() {
        return Err(mutation_error(
            mutation,
            "DELETE mutation requires a WHERE clause",
        ));
    }
    if !delete.tables.is_empty()
        || delete.using.is_some()
        || delete.returning.is_some()
        || delete.output.is_some()
        || !single_delete_from_table(&delete.from)
    {
        return Err(mutation_error(
            mutation,
            "unsupported multi-table DELETE; initial mutation support only accepts single-table DELETE",
        ));
    }

    Ok(())
}

fn single_delete_from_table(from: &FromTable) -> bool {
    match from {
        FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => {
            let [table] = tables.as_slice() else {
                return false;
            };
            is_single_table_with_optional_alias(table)
        }
    }
}

const fn is_single_table_with_optional_alias(table: &TableWithJoins) -> bool {
    table.joins.is_empty() && matches!(&table.relation, TableFactor::Table { args: None, .. })
}

fn query_is_values(query: &Query) -> bool {
    match query.body.as_ref() {
        SetExpr::Values(_) => true,
        SetExpr::Query(query) => query_is_values(query),
        SetExpr::Select(_)
        | SetExpr::SetOperation { .. }
        | SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => false,
    }
}

fn unsupported_insert_or_replace_form(
    mutation: &core::RawMutation,
    kind: core::MutationKind,
) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        format!(
            "unsupported {} mutation form; initial mutation support accepts {}",
            mutation_kind_keyword(kind),
            insert_or_replace_supported_forms(kind),
        ),
    )
}

fn insert_or_replace_select_error(
    mutation: &core::RawMutation,
    kind: core::MutationKind,
) -> core::DiagnosticReport {
    mutation_error(
        mutation,
        format!(
            "unsupported {} ... SELECT; initial mutation support accepts {}",
            mutation_kind_keyword(kind),
            insert_or_replace_supported_forms(kind),
        ),
    )
}

fn insert_or_replace_supported_forms(kind: core::MutationKind) -> &'static str {
    match kind {
        core::MutationKind::Insert => "INSERT ... VALUES and INSERT ... SET",
        core::MutationKind::Replace => "REPLACE ... VALUES and REPLACE ... SET",
        core::MutationKind::Update | core::MutationKind::Delete => unreachable!(),
    }
}

const fn mutation_kind_keyword(kind: core::MutationKind) -> &'static str {
    match kind {
        core::MutationKind::Insert => "INSERT",
        core::MutationKind::Update => "UPDATE",
        core::MutationKind::Delete => "DELETE",
        core::MutationKind::Replace => "REPLACE",
    }
}

fn tokenize_mutation(mutation: &core::RawMutation) -> core::DiagnosticResult<Vec<Token>> {
    let dialect = MySqlDialect {};
    Tokenizer::new(&dialect, mutation.analysis_sql())
        .tokenize()
        .map_err(|error| mutation_error(mutation, format!("failed to parse MySQL SQL: {error}")))
}

fn reject_unsupported_mutation_placeholders(
    mutation: &core::RawMutation,
    tokens: &[Token],
) -> core::DiagnosticResult<()> {
    let placeholder_count = tokens
        .iter()
        .filter(|token| matches!(token, Token::Placeholder(_)))
        .count();
    if placeholder_count == 0 {
        return Ok(());
    }

    let param_usage_count = mutation.param_usages().len();
    if param_usage_count == 0 {
        return Err(mutation_error(mutation, RAW_PLACEHOLDER_GUIDANCE));
    }
    if placeholder_count != param_usage_count {
        return Err(mutation_error(
            mutation,
            format!(
                "generated placeholder count {placeholder_count} does not match Param usage count {param_usage_count}"
            ),
        ));
    }

    Ok(())
}

fn validate_mutation_param_sample_expressions(
    mutation: &core::RawMutation,
) -> core::DiagnosticResult<()> {
    for usage in mutation.param_usages() {
        validate_mutation_param_sample_expression(mutation, usage)?;
    }

    Ok(())
}

fn validate_mutation_param_sample_expression(
    mutation: &core::RawMutation,
    usage: &core::ParamUsage,
) -> core::DiagnosticResult<()> {
    let trimmed = usage.sample_sql().trim();
    if trimmed.is_empty() {
        return Err(mutation_param_usage_error(
            mutation,
            usage,
            "Param range must contain exactly one SQL expression",
        ));
    }

    let dialect = MySqlDialect {};
    let mut parser = Parser::new(&dialect).try_with_sql(trimmed).map_err(|_| {
        mutation_param_usage_error(
            mutation,
            usage,
            "Param range must contain exactly one SQL expression",
        )
    })?;
    parser.parse_expr().map_err(|_| {
        mutation_param_usage_error(
            mutation,
            usage,
            "Param range must contain exactly one SQL expression",
        )
    })?;

    if parser.peek_token_ref().token != Token::EOF {
        return Err(mutation_param_usage_error(
            mutation,
            usage,
            "Param range must contain exactly one SQL expression",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::MysqlDialectAnalyzer;
    use sqlay_core as core;

    #[test]
    fn accepts_supported_mutation_statement_forms() {
        let cases = [
            (
                "INSERT INTO users (email, name) VALUES (?, ?);",
                core::MutationKind::Insert,
            ),
            (
                "INSERT INTO users SET email = ?, name = ?;",
                core::MutationKind::Insert,
            ),
            (
                "INSERT INTO users (email, name) VALUES (?, ?) ON DUPLICATE KEY UPDATE name = ?;",
                core::MutationKind::Insert,
            ),
            (
                "REPLACE INTO users (id, email) VALUES (?, ?);",
                core::MutationKind::Replace,
            ),
            (
                "REPLACE INTO users SET id = ?, email = ?;",
                core::MutationKind::Replace,
            ),
            (
                "UPDATE users AS u SET u.name = ? WHERE u.id = ? ORDER BY u.id LIMIT 1;",
                core::MutationKind::Update,
            ),
            (
                "DELETE FROM users AS u WHERE u.id = ? ORDER BY u.id LIMIT 1;",
                core::MutationKind::Delete,
            ),
        ];

        for (sql, expected_kind) in cases {
            let mutation =
                raw_mutation(sql).with_param_usages(param_usages(sql.matches('?').count()));

            let analysis = MysqlDialectAnalyzer
                .analyze_mutation(&mutation)
                .unwrap_or_else(|report| panic!("{sql}: {}", report.diagnostics()[0].message()));

            assert_eq!(analysis.kind(), expected_kind, "{sql}");
        }
    }

    #[test]
    fn accepts_subqueries_inside_supported_mutation_statements() {
        let mutation = raw_mutation(
            "UPDATE users AS u SET u.name = ? WHERE u.id IN (SELECT a.user_id FROM accounts AS a WHERE a.active = ?);",
        )
        .with_param_usages(vec![
            core::ParamUsage::new(
                "name".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            )
            .with_sample_sql("'Ada'".to_owned()),
            core::ParamUsage::new(
                "active".to_owned(),
                Some(core::CoreType::Bool),
                false,
                core::SourceLocation::unknown(),
            )
            .with_sample_sql("1".to_owned()),
        ]);

        let analysis = MysqlDialectAnalyzer
            .analyze_mutation(&mutation)
            .expect("supported mutation should allow subqueries in expressions");

        assert_eq!(analysis.kind(), core::MutationKind::Update);
    }

    #[test]
    fn rejects_mutation_raw_positional_placeholders_without_param_usages() {
        let report = MysqlDialectAnalyzer
            .analyze_mutation(&raw_mutation("UPDATE users SET name = ? WHERE id = 1;"))
            .expect_err("raw mutation parameters should be rejected");

        assert_eq!(
            report.diagnostics()[0].message(),
            "raw `?` placeholders are not supported in source SQL; use paired `@sqlay` Param markers around a sample expression, such as `/* @sqlay { type: param id: value } */ 1 /* @sqlay { type: paramEnd } */`"
        );
    }

    #[test]
    fn rejects_unsupported_mutation_forms() {
        let cases = [
            (
                "UPDATE users SET name = 'Ada';",
                "UPDATE mutation requires a WHERE clause",
            ),
            (
                "DELETE FROM users;",
                "DELETE mutation requires a WHERE clause",
            ),
            (
                "UPDATE users AS u JOIN accounts AS a ON a.user_id = u.id SET u.name = 'Ada' WHERE a.id = 1;",
                "unsupported multi-table UPDATE; initial mutation support only accepts single-table UPDATE",
            ),
            (
                "DELETE u FROM users AS u JOIN accounts AS a ON a.user_id = u.id WHERE a.id = 1;",
                "unsupported multi-table DELETE; initial mutation support only accepts single-table DELETE",
            ),
            (
                "INSERT INTO archived_users (id) SELECT id FROM users;",
                "unsupported INSERT ... SELECT; initial mutation support accepts INSERT ... VALUES and INSERT ... SET",
            ),
            (
                "REPLACE INTO archived_users (id) SELECT id FROM users;",
                "unsupported REPLACE ... SELECT; initial mutation support accepts REPLACE ... VALUES and REPLACE ... SET",
            ),
            (
                "CALL refresh_users();",
                "unsupported mutation SQL statement `CALL`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
            ),
            (
                "WITH stale_users AS (SELECT id FROM users) UPDATE users SET name = 'Ada' WHERE id = 1;",
                "unsupported mutation SQL statement `WITH`; supported statement kinds are `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`",
            ),
        ];

        for (sql, expected_message) in cases {
            let report = MysqlDialectAnalyzer
                .analyze_mutation(&raw_mutation(sql))
                .unwrap_err_or_else(|_| panic!("{sql} should be rejected"));

            assert_eq!(report.diagnostics()[0].message(), expected_message, "{sql}");
        }
    }

    fn raw_mutation(sql: &str) -> core::RawMutation {
        core::RawMutation::new(
            core::MutationMetadata::new("testMutation".to_owned()),
            sql.to_owned(),
        )
    }

    fn param_usages(count: usize) -> Vec<core::ParamUsage> {
        (0..count)
            .map(|index| {
                core::ParamUsage::new(
                    format!("value{index}"),
                    Some(core::CoreType::String),
                    false,
                    core::SourceLocation::unknown(),
                )
                .with_sample_sql("'value'".to_owned())
            })
            .collect()
    }

    trait UnwrapErrOrElse<T, E> {
        fn unwrap_err_or_else(self, op: impl FnOnce(T) -> E) -> E;
    }

    impl<T, E> UnwrapErrOrElse<T, E> for Result<T, E> {
        fn unwrap_err_or_else(self, op: impl FnOnce(T) -> E) -> E {
            match self {
                Ok(value) => op(value),
                Err(error) => error,
            }
        }
    }
}
