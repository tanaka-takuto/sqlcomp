/* @sqlcomp
{
  type: query
  id: paramEndWithoutStart
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = 1
  /* @sqlcomp { type: paramEnd } */;
