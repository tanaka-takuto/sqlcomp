/* @sqlcomp
{
  type: query
  id: typeMetadataDirectColumns
}
*/
SELECT *
FROM fixture_all_column_type;

/* @sqlcomp
{
  type: query
  id: typeMetadataJoinColumns
}
*/
SELECT
  p.bigint_nn_col AS parentBigintNnCol,
  p.varchar_255_nn_col AS parentVarchar255NnCol,
  c.child_bigint_nn_col AS childBigintNnCol,
  c.varchar_32_nn_col AS childVarchar32NnCol,
  c.decimal_12_2_nn_col AS childDecimal12_2NnCol,
  c.double_col AS childDoubleCol,
  c.datetime_col AS childDatetimeCol,
  c.timestamp_col AS childTimestampCol,
  c.time_col AS childTimeCol,
  c.json_col AS childJsonCol
FROM fixture_all_column_type AS p
INNER JOIN fixture_child AS c
  ON c.parent_bigint_nn_col = p.bigint_nn_col;

/* @sqlcomp
{
  type: query
  id: typeMetadataLeftJoinColumns
}
*/
SELECT
  p.bigint_nn_col AS parentBigintNnCol,
  p.varchar_255_nn_col AS parentVarchar255NnCol,
  c.varchar_32_nn_col AS childVarchar32NnCol,
  c.decimal_12_2_nn_col AS childDecimal12_2NnCol,
  c.datetime_col AS childDatetimeCol,
  c.timestamp_col AS childTimestampCol,
  c.time_col AS childTimeCol,
  c.json_col AS childJsonCol
FROM fixture_all_column_type AS p
LEFT JOIN fixture_child AS c
  ON c.parent_bigint_nn_col = p.bigint_nn_col;

/* @sqlcomp
{
  type: query
  id: typeMetadataExpressions
}
*/
SELECT
  p.bigint_nn_col + 1 AS nextBigintNnCol,
  CONCAT(p.varchar_255_nn_col, ':', p.varchar_320_nn_col) AS concatVarcharCol,
  COALESCE(p.varchar_255_col, p.varchar_255_nn_col) AS coalesceVarcharCol,
  p.decimal_18_4_nn_col + c.decimal_12_2_nn_col AS decimalExpressionCol,
  c.double_col * 2 AS doubleExpressionCol,
  p.timestamp_col IS NULL AS timestampIsNullCol,
  JSON_EXTRACT(p.json_col, '$.tier') AS jsonExtractCol
FROM fixture_all_column_type AS p
LEFT JOIN fixture_child AS c
  ON c.parent_bigint_nn_col = p.bigint_nn_col;

/* @sqlcomp
{
  type: query
  id: typeMetadataAggregateExpressions
}
*/
SELECT
  p.bool_col AS boolCol,
  COUNT(*) AS countStarCol,
  SUM(p.int_nn_col) AS sumIntNnCol,
  CASE
    WHEN p.bool_col = 1 THEN 'one'
    ELSE 'zero'
  END AS caseVarcharCol
FROM fixture_all_column_type AS p
GROUP BY p.bool_col;

/* @sqlcomp
{
  type: query
  id: typeMetadataOddColumnNames
}
*/
SELECT
  bigint_nn_col AS `bigint nn col`,
  varchar_255_nn_col AS `varchar 255 nn col`,
  varchar_320_nn_col AS `class`
FROM fixture_all_column_type;

/* @sqlcomp
{
  type: query
  id: typeMetadataSingleRow
  cardinality: one
}
*/
SELECT
  bigint_nn_col AS bigintNnCol,
  varchar_255_nn_col AS varchar255NnCol,
  varchar_255_col AS varchar255Col
FROM fixture_all_column_type
WHERE bigint_nn_col = 1
LIMIT 1;
