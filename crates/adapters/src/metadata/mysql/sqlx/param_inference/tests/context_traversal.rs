use super::*;

#[test]
fn query_value_type_overrides_preserve_order_through_unsupported_clauses() {
    let sql = "WITH seed AS (SELECT ? AS marker) \
        SELECT ? AS select_marker, CASE WHEN ? THEN ? ELSE ? END AS case_marker, u.id \
        FROM users AS u \
        JOIN (SELECT ? AS derived_marker) AS d ON d.derived_marker = u.id \
        WHERE COALESCE(?, u.email) = u.email \
          AND u.email LIKE ? \
          AND ? BETWEEN ? AND ? \
          AND EXISTS (SELECT 1 FROM accounts AS a WHERE a.user_id = u.id AND ? = ?) \
          AND u.id IN (SELECT ? FROM accounts AS a WHERE a.user_id = u.id) \
          AND u.email = ? \
        GROUP BY ? \
        HAVING ? \
        ORDER BY ? \
        LIMIT ?;";
    let specs = [
        ("cteMarker", Some(core::CoreType::String)),
        ("selectMarker", Some(core::CoreType::String)),
        ("caseCondition", Some(core::CoreType::Bool)),
        ("caseThen", Some(core::CoreType::String)),
        ("caseElse", Some(core::CoreType::String)),
        ("derivedMarker", Some(core::CoreType::Int64)),
        ("coalesceEmail", Some(core::CoreType::String)),
        ("likePattern", Some(core::CoreType::String)),
        ("betweenValue", Some(core::CoreType::Int64)),
        ("betweenLow", Some(core::CoreType::Int64)),
        ("betweenHigh", Some(core::CoreType::Int64)),
        ("existsLeft", Some(core::CoreType::Int64)),
        ("existsRight", Some(core::CoreType::Int64)),
        ("inSubqueryValue", Some(core::CoreType::Int64)),
        ("email", None),
        ("groupMarker", Some(core::CoreType::String)),
        ("havingMarker", Some(core::CoreType::Bool)),
        ("orderMarker", Some(core::CoreType::String)),
        ("limitMarker", Some(core::CoreType::Int32)),
    ];
    let query = raw_param_query(sql, specs.iter().map(|(id, ty)| param_usage(id, *ty)));
    let schema_columns = [
        schema_column("users", "id", core::CoreType::Int64),
        schema_column("users", "email", core::CoreType::String),
    ];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("valueType overrides should preserve Param order through unsupported clauses");

    assert_eq!(params, expected_params(&specs, core::CoreType::String));
}

#[test]
fn query_value_type_overrides_preserve_order_inside_unsupported_cte_expressions() {
    let sql = "WITH seed AS ( \
            SELECT (? + ?) AS binary_marker, \
                   (? IS DISTINCT FROM ?) AS distinct_marker, \
                   (? IN (?, ?)) AS in_marker, \
                   (? BETWEEN ? AND ?) AS between_marker, \
                   (? LIKE ?) AS like_marker, \
                   CAST(? AS SIGNED) AS cast_marker, \
                   POSITION(? IN ?) AS position_marker, \
                   SUBSTRING(? FROM ? FOR ?) AS substring_marker, \
                   TRIM(BOTH ? FROM ?) AS trim_marker, \
                   COALESCE(?, ?) AS function_marker, \
                   CASE WHEN ? THEN ? ELSE ? END AS case_marker, \
                   EXISTS (SELECT ?) AS exists_marker \
        ) \
        SELECT u.id FROM users AS u WHERE u.email = ?;";
    let specs = [
        ("binaryLeft", Some(core::CoreType::Int64)),
        ("binaryRight", Some(core::CoreType::Int64)),
        ("distinctLeft", Some(core::CoreType::Int64)),
        ("distinctRight", Some(core::CoreType::Int64)),
        ("inValue", Some(core::CoreType::Int64)),
        ("inLeft", Some(core::CoreType::Int64)),
        ("inRight", Some(core::CoreType::Int64)),
        ("betweenValue", Some(core::CoreType::Int64)),
        ("betweenLow", Some(core::CoreType::Int64)),
        ("betweenHigh", Some(core::CoreType::Int64)),
        ("likeValue", Some(core::CoreType::String)),
        ("likePattern", Some(core::CoreType::String)),
        ("castValue", Some(core::CoreType::Int64)),
        ("positionNeedle", Some(core::CoreType::String)),
        ("positionHaystack", Some(core::CoreType::String)),
        ("substringValue", Some(core::CoreType::String)),
        ("substringFrom", Some(core::CoreType::Int32)),
        ("substringFor", Some(core::CoreType::Int32)),
        ("trimChar", Some(core::CoreType::String)),
        ("trimValue", Some(core::CoreType::String)),
        ("functionLeft", Some(core::CoreType::String)),
        ("functionRight", Some(core::CoreType::String)),
        ("caseCondition", Some(core::CoreType::Bool)),
        ("caseThen", Some(core::CoreType::String)),
        ("caseElse", Some(core::CoreType::String)),
        ("existsMarker", Some(core::CoreType::Int64)),
        ("email", None),
    ];
    let query = raw_param_query(sql, specs.iter().map(|(id, ty)| param_usage(id, *ty)));
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("unsupported CTE expressions should preserve Param order");

    assert_eq!(params, expected_params(&specs, core::CoreType::String));
}

#[test]
fn query_value_type_overrides_preserve_order_through_expression_wrappers() {
    let sql = "SELECT \
            (? + ?) AS binary_marker, \
            ((?)) AS nested_marker, \
            -? AS unary_marker, \
            CAST(? AS SIGNED) AS cast_marker, \
            EXTRACT(YEAR FROM ?) AS extract_marker, \
            CEIL(?) AS ceil_marker, \
            FLOOR(?) AS floor_marker, \
            ? COLLATE utf8mb4_bin AS collate_marker, \
            ? IS TRUE AS is_true_marker, \
            ? IS NOT TRUE AS is_not_true_marker, \
            ? IS FALSE AS is_false_marker, \
            ? IS NOT FALSE AS is_not_false_marker, \
            ? IS NULL AS is_null_marker, \
            ? IS NOT NULL AS is_not_null_marker, \
            POSITION(? IN ?) AS position_marker, \
            SUBSTRING(? FROM ? FOR ?) AS substring_marker, \
            TRIM(BOTH ? FROM ?) AS trim_marker, \
            COALESCE(?, ?) AS function_marker, \
            CASE ? WHEN ? THEN ? ELSE ? END AS case_marker, \
            (?, ?) AS tuple_marker, \
            u.id \
        FROM users AS u \
        WHERE u.email = ?;";
    let specs = [
        ("binaryLeft", Some(core::CoreType::Int64)),
        ("binaryRight", Some(core::CoreType::Int64)),
        ("nestedMarker", Some(core::CoreType::String)),
        ("unaryMarker", Some(core::CoreType::Int64)),
        ("castValue", Some(core::CoreType::Int64)),
        ("extractValue", Some(core::CoreType::DateTime)),
        ("ceilValue", Some(core::CoreType::Float64)),
        ("floorValue", Some(core::CoreType::Float64)),
        ("collateValue", Some(core::CoreType::String)),
        ("isTrueValue", Some(core::CoreType::Bool)),
        ("isNotTrueValue", Some(core::CoreType::Bool)),
        ("isFalseValue", Some(core::CoreType::Bool)),
        ("isNotFalseValue", Some(core::CoreType::Bool)),
        ("isNullValue", Some(core::CoreType::String)),
        ("isNotNullValue", Some(core::CoreType::String)),
        ("positionNeedle", Some(core::CoreType::String)),
        ("positionHaystack", Some(core::CoreType::String)),
        ("substringValue", Some(core::CoreType::String)),
        ("substringFrom", Some(core::CoreType::Int32)),
        ("substringFor", Some(core::CoreType::Int32)),
        ("trimChar", Some(core::CoreType::String)),
        ("trimValue", Some(core::CoreType::String)),
        ("functionLeft", Some(core::CoreType::String)),
        ("functionRight", Some(core::CoreType::String)),
        ("caseOperand", Some(core::CoreType::String)),
        ("caseWhen", Some(core::CoreType::String)),
        ("caseThen", Some(core::CoreType::String)),
        ("caseElse", Some(core::CoreType::String)),
        ("tupleLeft", Some(core::CoreType::String)),
        ("tupleRight", Some(core::CoreType::String)),
        ("email", None),
    ];
    let query = raw_param_query(sql, specs.iter().map(|(id, ty)| param_usage(id, *ty)));
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("expression wrapper valueTypes should preserve Param order");

    assert_eq!(params, expected_params(&specs, core::CoreType::String));
}

#[test]
fn query_value_type_overrides_preserve_order_inside_unsupported_subquery_clauses() {
    let sql = "SELECT u.id \
        FROM users AS u \
        WHERE EXISTS ( \
            SELECT ? AS select_marker \
            FROM JSON_TABLE(?, '$[*]' COLUMNS (id BIGINT PATH '$.id')) AS jt \
            JOIN (SELECT ? AS derived_marker) AS d ON d.derived_marker = jt.id AND ? = ? \
            WHERE ? IN (?, ?) \
            GROUP BY ? \
            HAVING ? \
            ORDER BY ? \
            LIMIT ? OFFSET ? \
        ) \
        AND u.email = ?;";
    let specs = [
        ("selectMarker", Some(core::CoreType::String)),
        ("jsonRows", Some(core::CoreType::Json)),
        ("derivedMarker", Some(core::CoreType::Int64)),
        ("joinLeft", Some(core::CoreType::Int64)),
        ("joinRight", Some(core::CoreType::Int64)),
        ("inValue", Some(core::CoreType::Int64)),
        ("inLeft", Some(core::CoreType::Int64)),
        ("inRight", Some(core::CoreType::Int64)),
        ("groupMarker", Some(core::CoreType::String)),
        ("havingMarker", Some(core::CoreType::Bool)),
        ("orderMarker", Some(core::CoreType::String)),
        ("limitMarker", Some(core::CoreType::Int32)),
        ("offsetMarker", Some(core::CoreType::Int32)),
        ("email", None),
    ];
    let query = raw_param_query(sql, specs.iter().map(|(id, ty)| param_usage(id, *ty)));
    let schema_columns = [schema_column("users", "email", core::CoreType::String)];

    let params = resolve_param_usage_metadata(&query, &schema_columns)
        .expect("unsupported subquery clauses should preserve Param order");

    assert_eq!(params, expected_params(&specs, core::CoreType::String));
}

#[test]
fn mutation_value_type_overrides_preserve_order_through_expressions_and_subqueries() {
    let sql = "UPDATE users AS u \
        SET u.name = COALESCE(?, u.name), \
            u.updated_at = CASE WHEN ? THEN ? ELSE ? END, \
            u.score = u.score + ? \
        WHERE EXISTS (SELECT ? AS marker FROM accounts AS a WHERE a.user_id = u.id AND a.status = ?) \
          AND u.email = ? \
        ORDER BY ? \
        LIMIT ?;";
    let specs = [
        ("coalesceName", Some(core::CoreType::String)),
        ("caseCondition", Some(core::CoreType::Bool)),
        ("caseThen", Some(core::CoreType::DateTime)),
        ("caseElse", Some(core::CoreType::DateTime)),
        ("scoreDelta", Some(core::CoreType::Int32)),
        ("subqueryMarker", Some(core::CoreType::String)),
        ("accountStatus", None),
        ("email", None),
        ("orderMarker", Some(core::CoreType::String)),
        ("limitMarker", Some(core::CoreType::Int32)),
    ];
    let mutation = raw_param_mutation(sql, specs.iter().map(|(id, ty)| param_usage(id, *ty)));
    let schema_columns = [
        schema_column("users", "email", core::CoreType::String),
        schema_column("accounts", "status", core::CoreType::Bool),
    ];

    let params = resolve_mutation_param_usage_metadata(&mutation, &schema_columns)
        .expect("mutation expression and subquery Params should preserve source order");

    assert_eq!(
        params,
        [
            core::DbParamUsage::new("coalesceName".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("caseCondition".to_owned(), core::CoreType::Bool),
            core::DbParamUsage::new("caseThen".to_owned(), core::CoreType::DateTime),
            core::DbParamUsage::new("caseElse".to_owned(), core::CoreType::DateTime),
            core::DbParamUsage::new("scoreDelta".to_owned(), core::CoreType::Int32),
            core::DbParamUsage::new("subqueryMarker".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("accountStatus".to_owned(), core::CoreType::Bool),
            core::DbParamUsage::new("email".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("orderMarker".to_owned(), core::CoreType::String),
            core::DbParamUsage::new("limitMarker".to_owned(), core::CoreType::Int32),
        ]
    );
}

fn expected_params(
    specs: &[(&str, Option<core::CoreType>)],
    inferred_type: core::CoreType,
) -> Vec<core::DbParamUsage> {
    specs
        .iter()
        .map(|(id, ty)| core::DbParamUsage::new((*id).to_owned(), ty.unwrap_or(inferred_type)))
        .collect()
}
