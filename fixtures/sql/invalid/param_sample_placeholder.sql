/* @sqlay
{
  type: query
  id: paramSamplePlaceholder
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = /* @sqlay { type: param id: bigintValue } */
  ?
  /* @sqlay { type: paramEnd } */;
