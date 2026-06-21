/* @sqlay
{
  type: query
  id: generationEscapedSql
}
*/
SELECT
  varchar_255_nn_col AS varchar255NnCol,
  '"quoted"' AS doubleQuotedCol,
  'C:\\tmp\\sqlay' AS pathTextCol,
  '${notAParam}' AS templateLikeTextCol,
  JSON_OBJECT('tierPath', '$.tier') AS jsonObjectCol
FROM fixture_all_column_type
WHERE varchar_320_nn_col = 'varchar-320-a'
  AND char_16_nn_col IN ('state_a', 'state_b');

/* @sqlay
{
  type: query
  id: generationInferredSingleRow
}
*/
SELECT
  bigint_nn_col AS bigintNnCol,
  varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type
ORDER BY bigint_nn_col ASC
LIMIT 1;

/* @sqlay
{
  type: query
  id: generationExplicitOneOverridesMany
  cardinality: one
}
*/
SELECT
  bigint_nn_col AS bigintNnCol,
  varchar_255_nn_col AS varchar255NnCol
FROM fixture_all_column_type
ORDER BY bigint_nn_col ASC;

/* @sqlay
{
  type: query
  id: generationExplicitManyOverridesLimitOne
  cardinality: many
}
*/
SELECT
  bigint_nn_col AS bigintNnCol,
  varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type
ORDER BY bigint_nn_col ASC
LIMIT 1;
