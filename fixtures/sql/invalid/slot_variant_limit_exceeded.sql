/* @sqlcomp
{
  type: fragment
  id: limitTargetA
}
*/
  AND p.bigint_nn_col > 0

/* @sqlcomp
{
  type: fragment
  id: limitTargetB
}
*/
  AND p.bigint_nn_col > 1

/* @sqlcomp
{
  type: fragment
  id: limitTargetC
}
*/
  AND p.bigint_nn_col > 2

/* @sqlcomp
{
  type: fragment
  id: limitTargetD
}
*/
  AND p.bigint_nn_col > 3

/* @sqlcomp
{
  type: query
  id: slotVariantLimitExceeded
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlcomp { type: slot id: filterA targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */
/* @sqlcomp { type: slot id: filterB targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */
/* @sqlcomp { type: slot id: filterC targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */
/* @sqlcomp { type: slot id: filterD targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */;
