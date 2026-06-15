/* @sqlcomp
{
  type: query
  id: nestedPathMapping
}
*/
SELECT
  p.bigint_nn_col AS parentBigintNnCol,
  c.varchar_32_nn_col AS childVarchar32NnCol
FROM fixture_all_column_type AS p
INNER JOIN fixture_child AS c
  ON c.parent_bigint_nn_col = p.bigint_nn_col
WHERE c.varchar_32_nn_col = 'child-a';
