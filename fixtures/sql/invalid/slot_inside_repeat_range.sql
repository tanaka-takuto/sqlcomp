/* @sqlay
{
  type: query
  id: slotInsideRepeatRange
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IN (
  /* @sqlay { type: repeat id: ids separator: "," } */
  /* @sqlay { type: slot id: filter targets: [activeOnly] } */
  1
  /* @sqlay { type: repeatEnd } */
);
