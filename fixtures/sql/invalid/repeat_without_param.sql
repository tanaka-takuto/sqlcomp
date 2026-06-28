/* @sqlay
{
  type: query
  id: repeatWithoutParam
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IN (
  /* @sqlay { type: repeat id: ids separator: "," } */
  1
  /* @sqlay { type: repeatEnd } */
);
