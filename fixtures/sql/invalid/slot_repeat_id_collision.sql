/* @sqlay
{
  type: fragment
  id: activeOnly
}
*/
  AND p.bool_nn_col = TRUE

/* @sqlay
{
  type: query
  id: slotRepeatIdCollision
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IN (
  /* @sqlay { type: repeat id: filter separator: "," } */
  /* @sqlay { type: param id: id valueType: int64 } */
  1
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
/* @sqlay { type: slot id: filter targets: [activeOnly] } */;
