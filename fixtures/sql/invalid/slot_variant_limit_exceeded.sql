/* @sqlay
{
  type: fragment
  id: limitTargetA
}
*/
  AND p.bigint_nn_col > 0

/* @sqlay
{
  type: fragment
  id: limitTargetB
}
*/
  AND p.bigint_nn_col > 1

/* @sqlay
{
  type: fragment
  id: limitTargetC
}
*/
  AND p.bigint_nn_col > 2

/* @sqlay
{
  type: fragment
  id: limitTargetD
}
*/
  AND p.bigint_nn_col > 3

/* @sqlay
{
  type: query
  id: slotVariantLimitExceeded
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlay { type: slot id: filterA targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */
/* @sqlay { type: slot id: filterB targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */
/* @sqlay { type: slot id: filterC targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */
/* @sqlay { type: slot id: filterD targets: [limitTargetA, limitTargetB, limitTargetC, limitTargetD] } */;
