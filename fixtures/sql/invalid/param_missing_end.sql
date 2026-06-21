/* @sqlay
{
  type: query
  id: paramMissingEnd
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = /* @sqlay { type: param id: bigintValue } */
  1;
