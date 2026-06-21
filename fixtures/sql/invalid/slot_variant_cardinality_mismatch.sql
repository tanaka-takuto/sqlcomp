/* @sqlay
{
  type: fragment
  id: limitOne
}
*/
LIMIT 1

/* @sqlay
{
  type: query
  id: slotVariantCardinalityMismatch
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col > 0
/* @sqlay { type: slot id: limiter targets: [limitOne] } */;
