/* @sqlcomp
{
  type: fragment
  id: limitOne
}
*/
LIMIT 1

/* @sqlcomp
{
  type: query
  id: slotVariantCardinalityMismatch
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col > 0
/* @sqlcomp { type: slot id: limiter targets: [limitOne] } */;
