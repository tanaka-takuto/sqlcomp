//! sqlx-backed `MySQL` metadata adapter.

use std::collections::{BTreeMap, BTreeSet};

use sqlcomp_app::MetadataProvider;
use sqlcomp_core as core;
use sqlparser::ast::{
    BinaryOperator, Expr, FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr,
    JoinConstraint, JoinOperator, LimitClause, ObjectName, OrderBy, OrderByKind, Query as SqlQuery,
    Select, SelectItem, SetExpr, Statement, TableFactor, TableWithJoins, Value,
};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlx::{
    AssertSqlSafe, Column, Connection, Executor, MySqlConnection, Row, SqlSafeStr, TypeInfo,
};

/// sqlx-backed `MySQL` metadata provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlxMysqlMetadataProvider {
    database_url: String,
}

impl SqlxMysqlMetadataProvider {
    /// Build a provider for the configured `MySQL` database URL.
    #[must_use]
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }

    /// Configured database URL.
    #[must_use]
    pub fn database_url(&self) -> &str {
        &self.database_url
    }
}

impl MetadataProvider for SqlxMysqlMetadataProvider {
    fn describe(
        &self,
        query: &core::RawQuery,
        _analysis: &core::AnalyzedQuery,
    ) -> core::DiagnosticResult<core::DbQueryMetadata> {
        if tokio::runtime::Handle::try_current().is_ok() {
            describe_query_metadata_on_worker_thread(self.database_url().to_owned(), query.clone())
        } else {
            describe_query_metadata_blocking(self.database_url(), query)
        }
    }
}

fn describe_query_metadata_on_worker_thread(
    database_url: String,
    query: core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let error_query = query.clone();
    std::thread::spawn(move || describe_query_metadata_blocking(&database_url, &query))
        .join()
        .unwrap_or_else(|_| {
            Err(query_error(
                &error_query,
                "MySQL metadata worker thread panicked",
            ))
        })
}

fn describe_query_metadata_blocking(
    database_url: &str,
    query: &core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            query_error(
                query,
                format!("failed to create MySQL metadata runtime: {error}"),
            )
        })?;

    runtime.block_on(describe_query_metadata(database_url, query))
}

async fn describe_query_metadata(
    database_url: &str,
    query: &core::RawQuery,
) -> core::DiagnosticResult<core::DbQueryMetadata> {
    let mut connection = MySqlConnection::connect(database_url)
        .await
        .map_err(|error| {
            query_error(
                query,
                format!("failed to connect to MySQL database: {error}"),
            )
        })?;

    let param_usages = describe_param_usages(&mut connection, query).await?;

    // The dialect analyzer has already accepted this query as the MVP's single
    // SELECT statement shape. sqlx requires the assertion for dynamic SQL text.
    let description = connection
        .describe(AssertSqlSafe(query.analysis_sql().to_owned()).into_sql_str())
        .await
        .map_err(|error| query_error(query, format!("failed to describe MySQL query: {error}")))?;

    Ok(core::DbQueryMetadata::new(
        description
            .columns()
            .iter()
            .enumerate()
            .map(|(index, column)| {
                map_mysql_result_column_metadata(
                    column.name(),
                    column.type_info().name(),
                    description.nullable(index),
                )
            })
            .collect(),
    )
    .with_param_usages(param_usages))
}

async fn describe_param_usages(
    connection: &mut MySqlConnection,
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if query.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let schema_columns = fetch_current_database_schema_columns(connection, query).await?;
    resolve_param_usage_metadata(query, &schema_columns)
}

async fn fetch_current_database_schema_columns(
    connection: &mut MySqlConnection,
    query: &core::RawQuery,
) -> core::DiagnosticResult<Vec<MysqlSchemaColumn>> {
    let table_names = current_database_table_names(query)?;
    if table_names.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat_n("?", table_names.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT TABLE_NAME AS table_name, COLUMN_NAME AS column_name, COLUMN_TYPE AS column_type \
         FROM information_schema.columns \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN ({placeholders})"
    );
    let mut schema_query = sqlx::query(AssertSqlSafe(sql).into_sql_str());
    for table_name in &table_names {
        schema_query = schema_query.bind(table_name);
    }

    let rows = schema_query.fetch_all(connection).await.map_err(|error| {
        query_error(
            query,
            format!("failed to describe MySQL schema columns: {error}"),
        )
    })?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let table_name: String = row.get("table_name");
            let column_name: String = row.get("column_name");
            let column_type: String = row.get("column_type");

            MysqlSchemaColumn::new(
                table_name,
                column_name,
                mysql_type_name_to_core_type(&column_type),
            )
        })
        .collect())
}

fn query_error(query: &core::RawQuery, message: impl Into<String>) -> core::DiagnosticReport {
    let mut diagnostic = core::Diagnostic::error(message);
    if let Some(location) = query.source_location() {
        diagnostic = diagnostic.with_location(location.clone());
    }

    core::DiagnosticReport::new(diagnostic)
}

fn param_usage_error(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    message: impl Into<String>,
) -> core::DiagnosticReport {
    let location =
        if usage.source_location().range().is_some() || usage.source_location().path().is_some() {
            usage.source_location().clone()
        } else {
            query
                .source_location()
                .cloned()
                .unwrap_or_else(core::SourceLocation::unknown)
        };

    core::DiagnosticReport::new(core::Diagnostic::error(message).with_location(location))
}

/// Map one `MySQL` result column description into core metadata.
#[must_use]
pub fn map_mysql_result_column_metadata(
    name: &str,
    type_name: &str,
    nullable: Option<bool>,
) -> core::DbResultColumn {
    core::DbResultColumn::new(
        name.to_owned(),
        mysql_type_name_to_core_type(type_name),
        nullable,
    )
}

fn mysql_type_name_to_core_type(type_name: &str) -> core::CoreType {
    let normalized = normalized_mysql_type_name(type_name);
    let (base_type, is_unsigned) = normalized
        .strip_suffix(" UNSIGNED")
        .map_or((normalized.as_str(), false), |base_type| (base_type, true));

    match base_type {
        "BOOL" | "BOOLEAN" => core::CoreType::Bool,
        "INT" | "INTEGER" if is_unsigned => core::CoreType::Int64,
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" => core::CoreType::Int32,
        "BIGINT" if is_unsigned => core::CoreType::Unknown,
        "BIGINT" => core::CoreType::Int64,
        "DEC" | "DECIMAL" | "FIXED" | "NUMERIC" => core::CoreType::Decimal,
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" | "REAL" => core::CoreType::Float64,
        "CHAR" | "ENUM" | "LONGTEXT" | "MEDIUMTEXT" | "SET" | "TEXT" | "TINYTEXT" | "VARCHAR" => {
            core::CoreType::String
        }
        "BINARY" | "BLOB" | "LONGBLOB" | "MEDIUMBLOB" | "TINYBLOB" | "VARBINARY" => {
            core::CoreType::Bytes
        }
        "DATE" => core::CoreType::Date,
        "TIME" => core::CoreType::Time,
        "DATETIME" | "TIMESTAMP" => core::CoreType::DateTime,
        "JSON" => core::CoreType::Json,
        _ => core::CoreType::Unknown,
    }
}

fn normalized_mysql_type_name(type_name: &str) -> String {
    let mut without_precision = String::with_capacity(type_name.len());
    let mut precision_depth = 0_u8;

    for character in type_name.trim().chars() {
        match character {
            '(' => precision_depth = precision_depth.saturating_add(1),
            ')' if precision_depth > 0 => precision_depth -= 1,
            _ if precision_depth == 0 => without_precision.push(character),
            _ => {}
        }
    }

    let mut collapsed = String::with_capacity(without_precision.len());
    for word in without_precision.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }

    collapsed.to_ascii_uppercase()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MysqlSchemaColumn {
    table_name: String,
    column_name: String,
    ty: core::CoreType,
}

impl MysqlSchemaColumn {
    const fn new(table_name: String, column_name: String, ty: core::CoreType) -> Self {
        Self {
            table_name,
            column_name,
            ty,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SchemaColumnTypes {
    columns: BTreeMap<(String, String), core::CoreType>,
    tables: BTreeSet<String>,
}

impl SchemaColumnTypes {
    fn from_columns(columns: &[MysqlSchemaColumn]) -> Self {
        let mut schema = Self::default();
        for column in columns {
            schema.tables.insert(column.table_name.clone());
            schema.columns.insert(
                (column.table_name.clone(), column.column_name.clone()),
                column.ty,
            );
        }

        schema
    }

    fn get(&self, table_name: &str, column_name: &str) -> Option<core::CoreType> {
        self.columns
            .get(&(table_name.to_owned(), column_name.to_owned()))
            .copied()
    }

    fn has_table(&self, table_name: &str) -> bool {
        self.tables.contains(table_name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ColumnRef {
    qualifier: String,
    column: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TableResolution {
    CurrentDatabase { table_name: String },
    Unsupported,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SelectTableSources {
    by_qualifier: BTreeMap<String, TableResolution>,
    current_database_table_names: BTreeSet<String>,
}

impl SelectTableSources {
    fn insert_current_database_table(&mut self, table_name: String, alias: Option<String>) {
        self.current_database_table_names.insert(table_name.clone());
        self.by_qualifier.insert(
            table_name.clone(),
            TableResolution::CurrentDatabase {
                table_name: table_name.clone(),
            },
        );
        if let Some(alias) = alias {
            self.by_qualifier
                .insert(alias, TableResolution::CurrentDatabase { table_name });
        }
    }

    fn insert_unsupported_table(&mut self, table_name: Option<String>, alias: Option<String>) {
        if let Some(table_name) = table_name {
            self.by_qualifier
                .insert(table_name, TableResolution::Unsupported);
        }
        if let Some(alias) = alias {
            self.by_qualifier
                .insert(alias, TableResolution::Unsupported);
        }
    }

    fn resolve(&self, qualifier: &str) -> Option<&TableResolution> {
        self.by_qualifier.get(qualifier)
    }
}

fn resolve_param_usage_metadata(
    query: &core::RawQuery,
    schema_columns: &[MysqlSchemaColumn],
) -> core::DiagnosticResult<Vec<core::DbParamUsage>> {
    if query.param_usages().is_empty() {
        return Ok(Vec::new());
    }

    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    let mut contexts = collect_query_param_contexts(parsed_query, select);
    if contexts.len() > query.param_usages().len() {
        return Err(query_error(
            query,
            format!(
                "resolved Param context count {} does not match source Param usage count {}",
                contexts.len(),
                query.param_usages().len()
            ),
        ));
    }
    contexts.resize(query.param_usages().len(), None);

    let table_sources = select_table_sources(parsed_query, select);
    let schema = SchemaColumnTypes::from_columns(schema_columns);
    let mut params = Vec::with_capacity(query.param_usages().len());

    for (usage, context) in query.param_usages().iter().zip(contexts) {
        let ty = if let Some(value_type) = usage.value_type_override() {
            value_type
        } else {
            resolve_inferred_param_type(query, usage, context.as_ref(), &table_sources, &schema)?
        };
        params.push(core::DbParamUsage::new(usage.id().to_owned(), ty));
    }

    Ok(params)
}

fn resolve_inferred_param_type(
    query: &core::RawQuery,
    usage: &core::ParamUsage,
    context: Option<&ColumnRef>,
    table_sources: &SelectTableSources,
    schema: &SchemaColumnTypes,
) -> core::DiagnosticResult<core::CoreType> {
    let Some(column) = context else {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` requires `valueType` because no supported qualified column context was found",
                usage.id()
            ),
        ));
    };

    let Some(table) = table_sources.resolve(&column.qualifier) else {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` references unknown table alias `{}`",
                usage.id(),
                column.qualifier
            ),
        ));
    };

    let TableResolution::CurrentDatabase { table_name } = table else {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` requires `valueType` because table alias `{}` does not resolve to a current-database table",
                usage.id(),
                column.qualifier
            ),
        ));
    };

    if let Some(ty) = schema.get(table_name, &column.column) {
        return Ok(ty);
    }

    if !schema.has_table(table_name) {
        return Err(param_usage_error(
            query,
            usage,
            format!(
                "Param `{}` references unknown current-database table `{table_name}`",
                usage.id()
            ),
        ));
    }

    Err(param_usage_error(
        query,
        usage,
        format!(
            "Param `{}` references unknown current-database column `{table_name}.{}`",
            usage.id(),
            column.column
        ),
    ))
}

fn current_database_table_names(query: &core::RawQuery) -> core::DiagnosticResult<Vec<String>> {
    let statements = parse_query(query)?;
    let parsed_query = single_select_query(query, &statements)?;
    let select = select_from_query(parsed_query)
        .expect("single_select_query verifies this is a top-level SELECT query");
    Ok(select_table_sources(parsed_query, select)
        .current_database_table_names
        .into_iter()
        .collect())
}

fn parse_query(query: &core::RawQuery) -> core::DiagnosticResult<Vec<Statement>> {
    let dialect = MySqlDialect {};
    Parser::parse_sql(&dialect, query.analysis_sql())
        .map_err(|error| query_error(query, format!("failed to parse MySQL SQL: {error}")))
}

fn single_select_query<'a>(
    query: &core::RawQuery,
    statements: &'a [Statement],
) -> core::DiagnosticResult<&'a SqlQuery> {
    let [Statement::Query(parsed_query)] = statements else {
        return Err(query_error(
            query,
            "Param type inference requires exactly one SELECT query",
        ));
    };

    if select_from_query(parsed_query).is_none() {
        return Err(query_error(
            query,
            "Param type inference requires a top-level SELECT query",
        ));
    }

    Ok(parsed_query)
}

fn select_from_query(query: &SqlQuery) -> Option<&Select> {
    match query.body.as_ref() {
        SetExpr::Select(select) => Some(select),
        SetExpr::Query(query) => select_from_query(query),
        SetExpr::SetOperation { .. }
        | SetExpr::Values(_)
        | SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => None,
    }
}

fn select_table_sources(query: &SqlQuery, select: &Select) -> SelectTableSources {
    let cte_names = cte_names(query);
    let mut sources = SelectTableSources::default();
    for table in &select.from {
        collect_table_with_joins_sources(table, &mut sources, &cte_names);
    }

    sources
}

fn cte_names(query: &SqlQuery) -> BTreeSet<String> {
    query
        .with
        .as_ref()
        .map(|with| {
            with.cte_tables
                .iter()
                .map(|cte| cte.alias.name.value.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn collect_table_with_joins_sources(
    table: &TableWithJoins,
    sources: &mut SelectTableSources,
    cte_names: &BTreeSet<String>,
) {
    collect_table_factor_source(&table.relation, sources, cte_names);
    for join in &table.joins {
        collect_table_factor_source(&join.relation, sources, cte_names);
    }
}

fn collect_table_factor_source(
    table: &TableFactor,
    sources: &mut SelectTableSources,
    cte_names: &BTreeSet<String>,
) {
    match table {
        TableFactor::Table {
            name, alias, args, ..
        } => {
            let alias = alias.as_ref().map(|alias| alias.name.value.clone());
            let parts = object_name_parts(name);
            if parts.len() == 1 && cte_names.contains(&parts[0]) {
                sources.insert_unsupported_table(Some(parts[0].clone()), alias);
            } else if args.is_none() && parts.len() == 1 {
                sources.insert_current_database_table(parts[0].clone(), alias);
            } else {
                sources.insert_unsupported_table(parts.last().cloned(), alias);
            }
        }
        TableFactor::Derived { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::TableFunction { alias, .. } | TableFactor::Function { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::JsonTable { alias, .. } => {
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        TableFactor::NestedJoin {
            table_with_joins,
            alias,
        } => {
            collect_table_with_joins_sources(table_with_joins, sources, cte_names);
            sources.insert_unsupported_table(
                None,
                alias.as_ref().map(|alias| alias.name.value.clone()),
            );
        }
        _ => {}
    }
}

fn object_name_parts(name: &ObjectName) -> Vec<String> {
    name.0
        .iter()
        .filter_map(|part| part.as_ident().map(|ident| ident.value.clone()))
        .collect()
}

fn collect_query_param_contexts(query: &SqlQuery, select: &Select) -> Vec<Option<ColumnRef>> {
    let mut contexts = Vec::new();

    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_unsupported_query_param_contexts(&cte.query, &mut contexts);
        }
    }
    for item in &select.projection {
        collect_select_item_param_contexts(item, &mut contexts);
    }
    for table in &select.from {
        collect_table_with_joins_param_contexts(table, &mut contexts);
    }
    if let Some(selection) = &select.selection {
        collect_expr_param_contexts(selection, &mut contexts);
    }
    collect_group_by_param_contexts(&select.group_by, &mut contexts);
    if let Some(having) = &select.having {
        collect_expr_param_contexts(having, &mut contexts);
    }
    if let Some(order_by) = &query.order_by {
        collect_order_by_param_contexts(order_by, &mut contexts);
    }
    if let Some(limit_clause) = &query.limit_clause {
        collect_limit_clause_param_contexts(limit_clause, &mut contexts);
    }

    contexts
}

fn collect_select_item_param_contexts(item: &SelectItem, contexts: &mut Vec<Option<ColumnRef>>) {
    match item {
        SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } => collect_expr_param_contexts(expr, contexts),
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => {}
    }
}

fn collect_table_with_joins_param_contexts(
    table: &TableWithJoins,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    collect_table_factor_param_contexts(&table.relation, contexts);
    for join in &table.joins {
        collect_table_factor_param_contexts(&join.relation, contexts);
        if let Some(constraint) = join_constraint(&join.join_operator) {
            collect_join_constraint_param_contexts(constraint, contexts);
        }
    }
}

fn collect_table_factor_param_contexts(table: &TableFactor, contexts: &mut Vec<Option<ColumnRef>>) {
    match table {
        TableFactor::Derived { subquery, .. } => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        TableFactor::TableFunction { expr, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
        }
        TableFactor::JsonTable { json_expr, .. } => {
            collect_unsupported_expr_param_contexts(json_expr, contexts);
        }
        TableFactor::Function { args, .. } => {
            for arg in args {
                match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_unsupported_expr_param_contexts(name, contexts);
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                }
            }
        }
        TableFactor::UNNEST { array_exprs, .. } => {
            for expr in array_exprs {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            collect_table_with_joins_param_contexts(table_with_joins, contexts);
        }
        _ => {}
    }
}

const fn join_constraint(operator: &JoinOperator) -> Option<&JoinConstraint> {
    match operator {
        JoinOperator::Join(constraint)
        | JoinOperator::Inner(constraint)
        | JoinOperator::Left(constraint)
        | JoinOperator::LeftOuter(constraint)
        | JoinOperator::Right(constraint)
        | JoinOperator::RightOuter(constraint)
        | JoinOperator::FullOuter(constraint)
        | JoinOperator::CrossJoin(constraint)
        | JoinOperator::Semi(constraint)
        | JoinOperator::LeftSemi(constraint)
        | JoinOperator::RightSemi(constraint)
        | JoinOperator::Anti(constraint)
        | JoinOperator::LeftAnti(constraint)
        | JoinOperator::RightAnti(constraint)
        | JoinOperator::StraightJoin(constraint)
        | JoinOperator::AsOf { constraint, .. } => Some(constraint),
        JoinOperator::CrossApply
        | JoinOperator::OuterApply
        | JoinOperator::ArrayJoin
        | JoinOperator::LeftArrayJoin
        | JoinOperator::InnerArrayJoin => None,
    }
}

fn collect_join_constraint_param_contexts(
    constraint: &JoinConstraint,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let JoinConstraint::On(expr) = constraint {
        collect_expr_param_contexts(expr, contexts);
    }
}

fn collect_group_by_param_contexts(group_by: &GroupByExpr, contexts: &mut Vec<Option<ColumnRef>>) {
    match group_by {
        GroupByExpr::Expressions(expressions, _) => {
            for expr in expressions {
                collect_expr_param_contexts(expr, contexts);
            }
        }
        GroupByExpr::All(_) => {}
    }
}

fn collect_order_by_param_contexts(order_by: &OrderBy, contexts: &mut Vec<Option<ColumnRef>>) {
    match &order_by.kind {
        OrderByKind::Expressions(expressions) => {
            for order_by_expr in expressions {
                collect_expr_param_contexts(&order_by_expr.expr, contexts);
                if let Some(with_fill) = &order_by_expr.with_fill {
                    if let Some(from) = &with_fill.from {
                        collect_expr_param_contexts(from, contexts);
                    }
                    if let Some(to) = &with_fill.to {
                        collect_expr_param_contexts(to, contexts);
                    }
                    if let Some(step) = &with_fill.step {
                        collect_expr_param_contexts(step, contexts);
                    }
                }
            }
        }
        OrderByKind::All(_) => {}
    }
    if let Some(interpolate) = &order_by.interpolate
        && let Some(expressions) = &interpolate.exprs
    {
        for expr in expressions {
            if let Some(expr) = &expr.expr {
                collect_expr_param_contexts(expr, contexts);
            }
        }
    }
}

fn collect_limit_clause_param_contexts(
    limit_clause: &LimitClause,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match limit_clause {
        LimitClause::LimitOffset {
            limit,
            offset,
            limit_by,
        } => {
            if let Some(limit) = limit {
                collect_expr_param_contexts(limit, contexts);
            }
            if let Some(offset) = offset {
                collect_expr_param_contexts(&offset.value, contexts);
            }
            for expr in limit_by {
                collect_expr_param_contexts(expr, contexts);
            }
        }
        LimitClause::OffsetCommaLimit { offset, limit } => {
            collect_expr_param_contexts(offset, contexts);
            collect_expr_param_contexts(limit, contexts);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn collect_expr_param_contexts(expr: &Expr, contexts: &mut Vec<Option<ColumnRef>>) {
    if is_placeholder(expr) {
        contexts.push(None);
        return;
    }

    match expr {
        Expr::BinaryOp { left, op, right } if is_supported_comparison_operator(op) => {
            match (qualified_column_ref(left), is_placeholder(right)) {
                (Some(column), true) => contexts.push(Some(column)),
                _ => {
                    if let (true, Some(column)) =
                        (is_placeholder(left), qualified_column_ref(right))
                    {
                        contexts.push(Some(column));
                    } else {
                        collect_expr_param_contexts(left, contexts);
                        collect_expr_param_contexts(right, contexts);
                    }
                }
            }
        }
        Expr::BinaryOp { left, right, .. }
        | Expr::AnyOp { left, right, .. }
        | Expr::AllOp { left, right, .. }
        | Expr::IsDistinctFrom(left, right)
        | Expr::IsNotDistinctFrom(left, right) => {
            collect_expr_param_contexts(left, contexts);
            collect_expr_param_contexts(right, contexts);
        }
        Expr::InList {
            expr,
            list,
            negated,
        } if !negated => {
            if let Some(column) = qualified_column_ref(expr) {
                for item in list {
                    if is_placeholder(item) {
                        contexts.push(Some(column.clone()));
                    } else {
                        collect_expr_param_contexts(item, contexts);
                    }
                }
            } else {
                collect_expr_param_contexts(expr, contexts);
                for item in list {
                    collect_expr_param_contexts(item, contexts);
                }
            }
        }
        Expr::InList { expr, list, .. } => {
            collect_expr_param_contexts(expr, contexts);
            for item in list {
                collect_expr_param_contexts(item, contexts);
            }
        }
        Expr::InSubquery { expr, subquery, .. } => {
            collect_expr_param_contexts(expr, contexts);
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::InUnnest {
            expr, array_expr, ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(array_expr, contexts);
        }
        Expr::Nested(expr)
        | Expr::UnaryOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Extract { expr, .. }
        | Expr::Ceil { expr, .. }
        | Expr::Floor { expr, .. }
        | Expr::Collate { expr, .. }
        | Expr::Prefixed { value: expr, .. }
        | Expr::IsFalse(expr)
        | Expr::IsNotFalse(expr)
        | Expr::IsTrue(expr)
        | Expr::IsNotTrue(expr)
        | Expr::IsNull(expr)
        | Expr::IsNotNull(expr)
        | Expr::IsUnknown(expr)
        | Expr::IsNotUnknown(expr)
        | Expr::OuterJoin(expr)
        | Expr::Prior(expr)
        | Expr::Named { expr, .. } => collect_expr_param_contexts(expr, contexts),
        Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::Between {
            expr, low, high, ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(low, contexts);
            collect_expr_param_contexts(high, contexts);
        }
        Expr::Like { expr, pattern, .. }
        | Expr::ILike { expr, pattern, .. }
        | Expr::SimilarTo { expr, pattern, .. }
        | Expr::RLike { expr, pattern, .. } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(pattern, contexts);
        }
        Expr::Convert { expr, styles, .. } => {
            collect_expr_param_contexts(expr, contexts);
            for style in styles {
                collect_expr_param_contexts(style, contexts);
            }
        }
        Expr::AtTimeZone {
            timestamp,
            time_zone,
        } => {
            collect_expr_param_contexts(timestamp, contexts);
            collect_expr_param_contexts(time_zone, contexts);
        }
        Expr::Position { expr, r#in } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(r#in, contexts);
        }
        Expr::Substring {
            expr,
            substring_from,
            substring_for,
            ..
        } => {
            collect_expr_param_contexts(expr, contexts);
            if let Some(substring_from) = substring_from {
                collect_expr_param_contexts(substring_from, contexts);
            }
            if let Some(substring_for) = substring_for {
                collect_expr_param_contexts(substring_for, contexts);
            }
        }
        Expr::Trim {
            trim_what,
            expr,
            trim_characters,
            ..
        } => {
            if let Some(trim_what) = trim_what {
                collect_expr_param_contexts(trim_what, contexts);
            }
            collect_expr_param_contexts(expr, contexts);
            if let Some(trim_characters) = trim_characters {
                for character in trim_characters {
                    collect_expr_param_contexts(character, contexts);
                }
            }
        }
        Expr::Overlay {
            expr,
            overlay_what,
            overlay_from,
            overlay_for,
        } => {
            collect_expr_param_contexts(expr, contexts);
            collect_expr_param_contexts(overlay_what, contexts);
            collect_expr_param_contexts(overlay_from, contexts);
            if let Some(overlay_for) = overlay_for {
                collect_expr_param_contexts(overlay_for, contexts);
            }
        }
        Expr::Function(function) => {
            collect_function_arguments_param_contexts(&function.parameters, contexts);
            collect_function_arguments_param_contexts(&function.args, contexts);
            if let Some(filter) = &function.filter {
                collect_expr_param_contexts(filter, contexts);
            }
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            if let Some(operand) = operand {
                collect_expr_param_contexts(operand, contexts);
            }
            for condition in conditions {
                collect_expr_param_contexts(&condition.condition, contexts);
                collect_expr_param_contexts(&condition.result, contexts);
            }
            if let Some(else_result) = else_result {
                collect_expr_param_contexts(else_result, contexts);
            }
        }
        Expr::GroupingSets(items) | Expr::Cube(items) | Expr::Rollup(items) => {
            for item in items {
                for expr in item {
                    collect_expr_param_contexts(expr, contexts);
                }
            }
        }
        Expr::Tuple(items) => {
            for item in items {
                collect_expr_param_contexts(item, contexts);
            }
        }
        Expr::Struct { values, .. } => {
            for value in values {
                collect_expr_param_contexts(value, contexts);
            }
        }
        _ => {}
    }
}

fn collect_function_arguments_param_contexts(
    arguments: &FunctionArguments,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match arguments {
        FunctionArguments::None => {}
        FunctionArguments::Subquery(query) => {
            collect_unsupported_query_param_contexts(query, contexts);
        }
        FunctionArguments::List(list) => {
            for arg in &list.args {
                match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        collect_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_expr_param_contexts(name, contexts);
                        collect_function_arg_expr_param_contexts(arg, contexts);
                    }
                }
            }
        }
    }
}

fn collect_function_arg_expr_param_contexts(
    arg: &FunctionArgExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let FunctionArgExpr::Expr(expr) = arg {
        collect_expr_param_contexts(expr, contexts);
    }
}

fn collect_unsupported_query_param_contexts(
    query: &SqlQuery,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_unsupported_query_param_contexts(&cte.query, contexts);
        }
    }
    collect_unsupported_set_expr_param_contexts(&query.body, contexts);
    if let Some(order_by) = &query.order_by {
        collect_unsupported_order_by_param_contexts(order_by, contexts);
    }
    if let Some(limit_clause) = &query.limit_clause {
        collect_unsupported_limit_clause_param_contexts(limit_clause, contexts);
    }
}

fn collect_unsupported_set_expr_param_contexts(
    expression: &SetExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match expression {
        SetExpr::Select(select) => collect_unsupported_select_param_contexts(select, contexts),
        SetExpr::Query(query) => collect_unsupported_query_param_contexts(query, contexts),
        SetExpr::SetOperation { left, right, .. } => {
            collect_unsupported_set_expr_param_contexts(left, contexts);
            collect_unsupported_set_expr_param_contexts(right, contexts);
        }
        SetExpr::Values(values) => {
            for row in &values.rows {
                for expr in row.iter() {
                    collect_unsupported_expr_param_contexts(expr, contexts);
                }
            }
        }
        SetExpr::Insert(_)
        | SetExpr::Update(_)
        | SetExpr::Delete(_)
        | SetExpr::Merge(_)
        | SetExpr::Table(_) => {}
    }
}

fn collect_unsupported_select_param_contexts(
    select: &Select,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    for item in &select.projection {
        collect_unsupported_select_item_param_contexts(item, contexts);
    }
    for table in &select.from {
        collect_unsupported_table_with_joins_param_contexts(table, contexts);
    }
    if let Some(selection) = &select.selection {
        collect_unsupported_expr_param_contexts(selection, contexts);
    }
    collect_unsupported_group_by_param_contexts(&select.group_by, contexts);
    if let Some(having) = &select.having {
        collect_unsupported_expr_param_contexts(having, contexts);
    }
}

fn collect_unsupported_select_item_param_contexts(
    item: &SelectItem,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match item {
        SelectItem::UnnamedExpr(expr)
        | SelectItem::ExprWithAlias { expr, .. }
        | SelectItem::ExprWithAliases { expr, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
        }
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => {}
    }
}

fn collect_unsupported_table_with_joins_param_contexts(
    table: &TableWithJoins,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    collect_unsupported_table_factor_param_contexts(&table.relation, contexts);
    for join in &table.joins {
        collect_unsupported_table_factor_param_contexts(&join.relation, contexts);
        if let Some(constraint) = join_constraint(&join.join_operator) {
            collect_unsupported_join_constraint_param_contexts(constraint, contexts);
        }
    }
}

fn collect_unsupported_table_factor_param_contexts(
    table: &TableFactor,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match table {
        TableFactor::Derived { subquery, .. } => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        TableFactor::TableFunction { expr, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
        }
        TableFactor::JsonTable { json_expr, .. } => {
            collect_unsupported_expr_param_contexts(json_expr, contexts);
        }
        TableFactor::Function { args, .. } => {
            for arg in args {
                match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_unsupported_expr_param_contexts(name, contexts);
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                }
            }
        }
        TableFactor::UNNEST { array_exprs, .. } => {
            for expr in array_exprs {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            collect_unsupported_table_with_joins_param_contexts(table_with_joins, contexts);
        }
        _ => {}
    }
}

fn collect_unsupported_join_constraint_param_contexts(
    constraint: &JoinConstraint,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let JoinConstraint::On(expr) = constraint {
        collect_unsupported_expr_param_contexts(expr, contexts);
    }
}

fn collect_unsupported_group_by_param_contexts(
    group_by: &GroupByExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match group_by {
        GroupByExpr::Expressions(expressions, _) => {
            for expr in expressions {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        GroupByExpr::All(_) => {}
    }
}

fn collect_unsupported_order_by_param_contexts(
    order_by: &OrderBy,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match &order_by.kind {
        OrderByKind::Expressions(expressions) => {
            for order_by_expr in expressions {
                collect_unsupported_expr_param_contexts(&order_by_expr.expr, contexts);
                if let Some(with_fill) = &order_by_expr.with_fill {
                    if let Some(from) = &with_fill.from {
                        collect_unsupported_expr_param_contexts(from, contexts);
                    }
                    if let Some(to) = &with_fill.to {
                        collect_unsupported_expr_param_contexts(to, contexts);
                    }
                    if let Some(step) = &with_fill.step {
                        collect_unsupported_expr_param_contexts(step, contexts);
                    }
                }
            }
        }
        OrderByKind::All(_) => {}
    }
    if let Some(interpolate) = &order_by.interpolate
        && let Some(expressions) = &interpolate.exprs
    {
        for expr in expressions {
            if let Some(expr) = &expr.expr {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
    }
}

fn collect_unsupported_limit_clause_param_contexts(
    limit_clause: &LimitClause,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match limit_clause {
        LimitClause::LimitOffset {
            limit,
            offset,
            limit_by,
        } => {
            if let Some(limit) = limit {
                collect_unsupported_expr_param_contexts(limit, contexts);
            }
            if let Some(offset) = offset {
                collect_unsupported_expr_param_contexts(&offset.value, contexts);
            }
            for expr in limit_by {
                collect_unsupported_expr_param_contexts(expr, contexts);
            }
        }
        LimitClause::OffsetCommaLimit { offset, limit } => {
            collect_unsupported_expr_param_contexts(offset, contexts);
            collect_unsupported_expr_param_contexts(limit, contexts);
        }
    }
}

#[allow(clippy::too_many_lines)]
fn collect_unsupported_expr_param_contexts(expr: &Expr, contexts: &mut Vec<Option<ColumnRef>>) {
    if is_placeholder(expr) {
        contexts.push(None);
        return;
    }

    match expr {
        Expr::BinaryOp { left, right, .. }
        | Expr::AnyOp { left, right, .. }
        | Expr::AllOp { left, right, .. }
        | Expr::IsDistinctFrom(left, right)
        | Expr::IsNotDistinctFrom(left, right) => {
            collect_unsupported_expr_param_contexts(left, contexts);
            collect_unsupported_expr_param_contexts(right, contexts);
        }
        Expr::InList { expr, list, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            for item in list {
                collect_unsupported_expr_param_contexts(item, contexts);
            }
        }
        Expr::InSubquery { expr, subquery, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::InUnnest {
            expr, array_expr, ..
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(array_expr, contexts);
        }
        Expr::Nested(expr)
        | Expr::UnaryOp { expr, .. }
        | Expr::Cast { expr, .. }
        | Expr::Extract { expr, .. }
        | Expr::Ceil { expr, .. }
        | Expr::Floor { expr, .. }
        | Expr::Collate { expr, .. }
        | Expr::Prefixed { value: expr, .. }
        | Expr::IsFalse(expr)
        | Expr::IsNotFalse(expr)
        | Expr::IsTrue(expr)
        | Expr::IsNotTrue(expr)
        | Expr::IsNull(expr)
        | Expr::IsNotNull(expr)
        | Expr::IsUnknown(expr)
        | Expr::IsNotUnknown(expr)
        | Expr::OuterJoin(expr)
        | Expr::Prior(expr)
        | Expr::Named { expr, .. } => collect_unsupported_expr_param_contexts(expr, contexts),
        Expr::Between {
            expr, low, high, ..
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(low, contexts);
            collect_unsupported_expr_param_contexts(high, contexts);
        }
        Expr::Like { expr, pattern, .. }
        | Expr::ILike { expr, pattern, .. }
        | Expr::SimilarTo { expr, pattern, .. }
        | Expr::RLike { expr, pattern, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(pattern, contexts);
        }
        Expr::Convert { expr, styles, .. } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            for style in styles {
                collect_unsupported_expr_param_contexts(style, contexts);
            }
        }
        Expr::AtTimeZone {
            timestamp,
            time_zone,
        } => {
            collect_unsupported_expr_param_contexts(timestamp, contexts);
            collect_unsupported_expr_param_contexts(time_zone, contexts);
        }
        Expr::Position { expr, r#in } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(r#in, contexts);
        }
        Expr::Substring {
            expr,
            substring_from,
            substring_for,
            ..
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            if let Some(substring_from) = substring_from {
                collect_unsupported_expr_param_contexts(substring_from, contexts);
            }
            if let Some(substring_for) = substring_for {
                collect_unsupported_expr_param_contexts(substring_for, contexts);
            }
        }
        Expr::Trim {
            trim_what,
            expr,
            trim_characters,
            ..
        } => {
            if let Some(trim_what) = trim_what {
                collect_unsupported_expr_param_contexts(trim_what, contexts);
            }
            collect_unsupported_expr_param_contexts(expr, contexts);
            if let Some(trim_characters) = trim_characters {
                for character in trim_characters {
                    collect_unsupported_expr_param_contexts(character, contexts);
                }
            }
        }
        Expr::Overlay {
            expr,
            overlay_what,
            overlay_from,
            overlay_for,
        } => {
            collect_unsupported_expr_param_contexts(expr, contexts);
            collect_unsupported_expr_param_contexts(overlay_what, contexts);
            collect_unsupported_expr_param_contexts(overlay_from, contexts);
            if let Some(overlay_for) = overlay_for {
                collect_unsupported_expr_param_contexts(overlay_for, contexts);
            }
        }
        Expr::Function(function) => {
            collect_unsupported_function_arguments_param_contexts(&function.parameters, contexts);
            collect_unsupported_function_arguments_param_contexts(&function.args, contexts);
            if let Some(filter) = &function.filter {
                collect_unsupported_expr_param_contexts(filter, contexts);
            }
        }
        Expr::Case {
            operand,
            conditions,
            else_result,
            ..
        } => {
            if let Some(operand) = operand {
                collect_unsupported_expr_param_contexts(operand, contexts);
            }
            for condition in conditions {
                collect_unsupported_expr_param_contexts(&condition.condition, contexts);
                collect_unsupported_expr_param_contexts(&condition.result, contexts);
            }
            if let Some(else_result) = else_result {
                collect_unsupported_expr_param_contexts(else_result, contexts);
            }
        }
        Expr::Exists { subquery, .. } | Expr::Subquery(subquery) => {
            collect_unsupported_query_param_contexts(subquery, contexts);
        }
        Expr::GroupingSets(items) | Expr::Cube(items) | Expr::Rollup(items) => {
            for item in items {
                for expr in item {
                    collect_unsupported_expr_param_contexts(expr, contexts);
                }
            }
        }
        Expr::Tuple(items) => {
            for item in items {
                collect_unsupported_expr_param_contexts(item, contexts);
            }
        }
        Expr::Struct { values, .. } => {
            for value in values {
                collect_unsupported_expr_param_contexts(value, contexts);
            }
        }
        _ => {}
    }
}

fn collect_unsupported_function_arguments_param_contexts(
    arguments: &FunctionArguments,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    match arguments {
        FunctionArguments::None => {}
        FunctionArguments::Subquery(query) => {
            collect_unsupported_query_param_contexts(query, contexts);
        }
        FunctionArguments::List(list) => {
            for arg in &list.args {
                match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                    FunctionArg::ExprNamed { name, arg, .. } => {
                        collect_unsupported_expr_param_contexts(name, contexts);
                        collect_unsupported_function_arg_expr_param_contexts(arg, contexts);
                    }
                }
            }
        }
    }
}

fn collect_unsupported_function_arg_expr_param_contexts(
    arg: &FunctionArgExpr,
    contexts: &mut Vec<Option<ColumnRef>>,
) {
    if let FunctionArgExpr::Expr(expr) = arg {
        collect_unsupported_expr_param_contexts(expr, contexts);
    }
}

const fn is_supported_comparison_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::LtEq
            | BinaryOperator::Gt
            | BinaryOperator::GtEq
    )
}

fn qualified_column_ref(expr: &Expr) -> Option<ColumnRef> {
    let Expr::CompoundIdentifier(parts) = expr else {
        return None;
    };
    let [qualifier, column] = parts.as_slice() else {
        return None;
    };

    Some(ColumnRef {
        qualifier: qualifier.value.clone(),
        column: column.value.clone(),
    })
}

fn is_placeholder(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(value) if matches!(&value.value, Value::Placeholder(value) if value == "?"))
}

#[cfg(test)]
mod tests {
    use super::{
        MysqlSchemaColumn, map_mysql_result_column_metadata, resolve_param_usage_metadata,
    };
    use sqlcomp_core as core;

    #[test]
    fn maps_representative_mysql_type_names_to_core_types() {
        let cases = [
            ("BOOLEAN", core::CoreType::Bool),
            ("TINYINT", core::CoreType::Int32),
            ("SMALLINT", core::CoreType::Int32),
            ("MEDIUMINT", core::CoreType::Int32),
            ("INT", core::CoreType::Int32),
            ("INTEGER", core::CoreType::Int32),
            ("BIGINT", core::CoreType::Int64),
            ("DECIMAL", core::CoreType::Decimal),
            ("NUMERIC", core::CoreType::Decimal),
            ("FLOAT", core::CoreType::Float64),
            ("DOUBLE", core::CoreType::Float64),
            ("REAL", core::CoreType::Float64),
            ("CHAR", core::CoreType::String),
            ("VARCHAR", core::CoreType::String),
            ("TEXT", core::CoreType::String),
            ("TINYTEXT", core::CoreType::String),
            ("MEDIUMTEXT", core::CoreType::String),
            ("LONGTEXT", core::CoreType::String),
            ("ENUM", core::CoreType::String),
            ("SET", core::CoreType::String),
            ("BINARY", core::CoreType::Bytes),
            ("VARBINARY", core::CoreType::Bytes),
            ("BLOB", core::CoreType::Bytes),
            ("TINYBLOB", core::CoreType::Bytes),
            ("MEDIUMBLOB", core::CoreType::Bytes),
            ("LONGBLOB", core::CoreType::Bytes),
            ("DATE", core::CoreType::Date),
            ("TIME", core::CoreType::Time),
            ("DATETIME", core::CoreType::DateTime),
            ("TIMESTAMP", core::CoreType::DateTime),
            ("JSON", core::CoreType::Json),
        ];

        for (type_name, expected_type) in cases {
            let column = map_mysql_result_column_metadata("value", type_name, Some(false));

            assert_eq!(
                column,
                core::DbResultColumn::new("value".to_owned(), expected_type, Some(false)),
                "{type_name} should map to {expected_type:?}"
            );
        }
    }

    #[test]
    fn maps_unknown_mysql_type_names_conservatively() {
        let column = map_mysql_result_column_metadata("shape", "GEOMETRY", Some(false));

        assert_eq!(
            column,
            core::DbResultColumn::new("shape".to_owned(), core::CoreType::Unknown, Some(false))
        );
    }

    #[test]
    fn preserves_mysql_nullability_metadata_for_core_ir() {
        let nullable = map_mysql_result_column_metadata("nickname", "VARCHAR", Some(true));
        let non_nullable = map_mysql_result_column_metadata("displayName", "VARCHAR", Some(false));

        assert_eq!(
            nullable,
            core::DbResultColumn::new("nickname".to_owned(), core::CoreType::String, Some(true))
        );
        assert!(nullable.to_result_column().is_nullable());

        assert_eq!(
            non_nullable,
            core::DbResultColumn::new(
                "displayName".to_owned(),
                core::CoreType::String,
                Some(false),
            )
        );
        assert!(!non_nullable.to_result_column().is_nullable());
    }

    #[test]
    fn preserves_unknown_nullability_for_core_ir() {
        let column = map_mysql_result_column_metadata("name", "VARCHAR", None);

        assert_eq!(
            column,
            core::DbResultColumn::new("name".to_owned(), core::CoreType::String, None)
        );
        assert!(column.to_result_column().is_nullable());
    }

    #[test]
    fn normalizes_case_and_precision_suffixes() {
        let column = map_mysql_result_column_metadata("amount", "decimal(18, 4)", Some(false));

        assert_eq!(
            column,
            core::DbResultColumn::new("amount".to_owned(), core::CoreType::Decimal, Some(false))
        );

        let widened = map_mysql_result_column_metadata("count", "int(10) unsigned", Some(false));

        assert_eq!(
            widened,
            core::DbResultColumn::new("count".to_owned(), core::CoreType::Int64, Some(false))
        );

        let unknown = map_mysql_result_column_metadata("id", "BIGINT UNSIGNED", Some(false));

        assert_eq!(
            unknown,
            core::DbResultColumn::new("id".to_owned(), core::CoreType::Unknown, Some(false))
        );
    }

    #[test]
    fn resolves_param_types_from_direct_qualified_column_contexts() {
        let query = raw_param_query(
            "SELECT u.id FROM users AS u JOIN accounts ON accounts.user_id = u.id WHERE u.email = ? AND accounts.id <> ? AND accounts.balance >= ? AND u.id IN (?, ?);",
            [
                core::ParamUsage::new(
                    "email".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "accountId".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "minimumBalance".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "primaryUserId".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "secondaryUserId".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
            ],
        );
        let schema_columns = [
            schema_column("users", "id", core::CoreType::Int64),
            schema_column("users", "email", core::CoreType::String),
            schema_column("accounts", "id", core::CoreType::Int64),
            schema_column("accounts", "balance", core::CoreType::Decimal),
        ];

        let params = resolve_param_usage_metadata(&query, &schema_columns)
            .expect("qualified direct column contexts should resolve");

        assert_eq!(
            params,
            [
                core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("accountId".to_owned(), core::CoreType::Int64),
                core::DbParamUsage::new("minimumBalance".to_owned(), core::CoreType::Decimal),
                core::DbParamUsage::new("primaryUserId".to_owned(), core::CoreType::Int64),
                core::DbParamUsage::new("secondaryUserId".to_owned(), core::CoreType::Int64),
            ]
        );
    }

    #[test]
    fn value_type_override_skips_direct_column_inference() {
        let query = raw_param_query(
            "SELECT u.id FROM users AS u WHERE unknown_alias.email = ?;",
            [core::ParamUsage::new(
                "email".to_owned(),
                Some(core::CoreType::String),
                false,
                core::SourceLocation::unknown(),
            )],
        );

        let params = resolve_param_usage_metadata(&query, &[])
            .expect("valueType override should not require schema inference");

        assert_eq!(
            params,
            [core::DbParamUsage::new(
                "email".to_owned(),
                core::CoreType::String
            )]
        );
    }

    #[test]
    fn value_type_override_allows_params_in_untraversed_query_clauses() {
        let query = raw_param_query(
            "SELECT COUNT(*) FROM users AS u GROUP BY ? ORDER BY ? LIMIT ?;",
            [
                core::ParamUsage::new(
                    "groupKey".to_owned(),
                    Some(core::CoreType::String),
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "sortKey".to_owned(),
                    Some(core::CoreType::String),
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "limitCount".to_owned(),
                    Some(core::CoreType::Int32),
                    false,
                    core::SourceLocation::unknown(),
                ),
            ],
        );
        let schema_columns = [schema_column("users", "id", core::CoreType::Int64)];

        let params = resolve_param_usage_metadata(&query, &schema_columns)
            .expect("valueType should allow unsupported query clause Param contexts");

        assert_eq!(
            params,
            [
                core::DbParamUsage::new("groupKey".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("sortKey".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("limitCount".to_owned(), core::CoreType::Int32),
            ]
        );
    }

    #[test]
    fn value_type_override_in_subquery_preserves_later_inference_order() {
        let query = raw_param_query(
            "SELECT (SELECT ?) AS marker FROM users AS u WHERE u.email = ?;",
            [
                core::ParamUsage::new(
                    "marker".to_owned(),
                    Some(core::CoreType::String),
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "email".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
            ],
        );
        let schema_columns = [schema_column("users", "email", core::CoreType::String)];

        let params = resolve_param_usage_metadata(&query, &schema_columns)
            .expect("subquery valueType should not shift later Param inference");

        assert_eq!(
            params,
            [
                core::DbParamUsage::new("marker".to_owned(), core::CoreType::String),
                core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
            ]
        );
    }

    #[test]
    fn value_type_override_in_json_table_preserves_later_inference_order() {
        let query = raw_param_query(
            "SELECT u.id FROM JSON_TABLE(?, '$[*]' COLUMNS (id BIGINT PATH '$.id')) AS jt JOIN users AS u ON u.id = jt.id WHERE u.email = ?;",
            [
                core::ParamUsage::new(
                    "jsonRows".to_owned(),
                    Some(core::CoreType::Json),
                    false,
                    core::SourceLocation::unknown(),
                ),
                core::ParamUsage::new(
                    "email".to_owned(),
                    None,
                    false,
                    core::SourceLocation::unknown(),
                ),
            ],
        );
        let schema_columns = [
            schema_column("users", "id", core::CoreType::Int64),
            schema_column("users", "email", core::CoreType::String),
        ];

        let params = resolve_param_usage_metadata(&query, &schema_columns)
            .expect("JSON_TABLE valueType should not shift later Param inference");

        assert_eq!(
            params,
            [
                core::DbParamUsage::new("jsonRows".to_owned(), core::CoreType::Json),
                core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
            ]
        );
    }

    #[test]
    fn rejects_param_without_value_type_when_context_is_not_supported() {
        let query = raw_param_query(
            "SELECT u.id FROM users AS u WHERE COALESCE(?, u.email) = u.email;",
            [core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );
        let schema_columns = [schema_column("users", "email", core::CoreType::String)];

        let report = resolve_param_usage_metadata(&query, &schema_columns)
            .expect_err("function context should require valueType");

        assert_eq!(
            report.diagnostics()[0].message(),
            "Param `email` requires `valueType` because no supported qualified column context was found"
        );
    }

    #[test]
    fn rejects_unqualified_column_inference_without_value_type() {
        let query = raw_param_query(
            "SELECT id FROM users WHERE email = ?;",
            [core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );
        let schema_columns = [schema_column("users", "email", core::CoreType::String)];

        let report = resolve_param_usage_metadata(&query, &schema_columns)
            .expect_err("unqualified columns should require valueType");

        assert_eq!(
            report.diagnostics()[0].message(),
            "Param `email` requires `valueType` because no supported qualified column context was found"
        );
    }

    #[test]
    fn rejects_schema_qualified_table_inference_without_value_type() {
        let query = raw_param_query(
            "SELECT u.id FROM app.users AS u WHERE u.email = ?;",
            [core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );

        let report = resolve_param_usage_metadata(&query, &[])
            .expect_err("schema-qualified tables should require valueType");

        assert_eq!(
            report.diagnostics()[0].message(),
            "Param `email` requires `valueType` because table alias `u` does not resolve to a current-database table"
        );
    }

    #[test]
    fn rejects_cte_source_shadowing_real_table_without_value_type() {
        let query = raw_param_query(
            "WITH u AS (SELECT id FROM users) SELECT u.id FROM u WHERE u.id = ?;",
            [core::ParamUsage::new(
                "userId".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );
        let schema_columns = [schema_column("u", "id", core::CoreType::Int64)];

        let report = resolve_param_usage_metadata(&query, &schema_columns)
            .expect_err("CTE names should shadow current-database tables");

        assert_eq!(
            report.diagnostics()[0].message(),
            "Param `userId` requires `valueType` because table alias `u` does not resolve to a current-database table"
        );
    }

    #[test]
    fn resolves_table_aliases_inside_nested_joins() {
        let query = raw_param_query(
            "SELECT u.id FROM (users AS u JOIN accounts AS a ON a.user_id = u.id) WHERE u.email = ?;",
            [core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );
        let schema_columns = [schema_column("users", "email", core::CoreType::String)];

        let params = resolve_param_usage_metadata(&query, &schema_columns)
            .expect("aliases inside parenthesized joins should resolve");

        assert_eq!(
            params,
            [core::DbParamUsage::new(
                "email".to_owned(),
                core::CoreType::String
            )]
        );
    }

    #[test]
    fn rejects_unknown_alias_without_value_type() {
        let query = raw_param_query(
            "SELECT u.id FROM users AS u WHERE missing.email = ?;",
            [core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );
        let schema_columns = [schema_column("users", "email", core::CoreType::String)];

        let report = resolve_param_usage_metadata(&query, &schema_columns)
            .expect_err("unknown table aliases should be diagnosed");

        assert_eq!(
            report.diagnostics()[0].message(),
            "Param `email` references unknown table alias `missing`"
        );
    }

    #[test]
    fn rejects_unknown_current_database_column_without_value_type() {
        let query = raw_param_query(
            "SELECT u.id FROM users AS u WHERE u.missing_email = ?;",
            [core::ParamUsage::new(
                "email".to_owned(),
                None,
                false,
                core::SourceLocation::unknown(),
            )],
        );
        let schema_columns = [schema_column("users", "id", core::CoreType::Int64)];

        let report = resolve_param_usage_metadata(&query, &schema_columns)
            .expect_err("unknown current-database columns should be diagnosed");

        assert_eq!(
            report.diagnostics()[0].message(),
            "Param `email` references unknown current-database column `users.missing_email`"
        );
    }

    fn raw_param_query(
        analysis_sql: &str,
        param_usages: impl IntoIterator<Item = core::ParamUsage>,
    ) -> core::RawQuery {
        core::RawQuery::new(
            core::QueryMetadata::new("findUsers".to_owned(), None),
            analysis_sql.to_owned(),
        )
        .with_analysis_sql(analysis_sql.to_owned())
        .with_param_usages(param_usages.into_iter().collect())
    }

    fn schema_column(table_name: &str, column_name: &str, ty: core::CoreType) -> MysqlSchemaColumn {
        MysqlSchemaColumn::new(table_name.to_owned(), column_name.to_owned(), ty)
    }
}
