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
  id: slotDuplicateTarget
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlay { type: slot id: filter targets: [activeOnly, activeOnly] } */;
