/* @sqlay
{
  type: query
  id: repeatNonStringSeparator
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IN (
  /* @sqlay { type: repeat id: ids separator: true } */
  /* @sqlay { type: param id: id valueType: int64 } */
  1
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
);
