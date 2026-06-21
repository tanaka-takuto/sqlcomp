/* @sqlcomp
{
  type: fragment
  id: activeOnly
}
*/
  AND p.bool_nn_col = TRUE

/* @sqlcomp
{
  type: fragment
  id: textOnly
}
*/
  AND p.varchar_320_nn_col IS NOT NULL

/* @sqlcomp
{
  type: query
  id: repeatedSlotSameTargetsDifferentOrder
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlcomp { type: slot id: filter targets: [activeOnly, textOnly] } */
  AND p.bigint_nn_col > 0
/* @sqlcomp { type: slot id: filter targets: [textOnly, activeOnly] } */;
