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
  id: directParamSlotIdCollision
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.varchar_320_nn_col = /* @sqlay { type: param id: filter } */
  'varchar-320-a'
  /* @sqlay { type: paramEnd } */
/* @sqlay { type: slot id: filter targets: [activeOnly] } */;
