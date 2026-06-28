/* @sqlay
{
  type: fragment
  id: repeatLimitTargetA
}
*/
  AND p.bigint_nn_col > 0

/* @sqlay
{
  type: fragment
  id: repeatLimitTargetB
}
*/
  AND p.bigint_nn_col > 1

/* @sqlay
{
  type: fragment
  id: repeatLimitTargetC
}
*/
  AND p.bigint_nn_col > 2

/* @sqlay
{
  type: fragment
  id: repeatLimitTargetD
}
*/
  AND p.bigint_nn_col > 3

/* @sqlay
{
  type: query
  id: repeatValidationCaseLimitExceeded
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IN (
  /* @sqlay { type: repeat id: ids separator: "," } */
  /* @sqlay { type: param id: id valueType: int64 } */
  1
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
/* @sqlay { type: slot id: filterA targets: [repeatLimitTargetA, repeatLimitTargetB, repeatLimitTargetC, repeatLimitTargetD] } */
/* @sqlay { type: slot id: filterB targets: [repeatLimitTargetA, repeatLimitTargetB, repeatLimitTargetC, repeatLimitTargetD] } */
/* @sqlay { type: slot id: filterC targets: [repeatLimitTargetA, repeatLimitTargetB, repeatLimitTargetC, repeatLimitTargetD] } */
/* @sqlay { type: slot id: filterD targets: [repeatLimitTargetA, repeatLimitTargetB, repeatLimitTargetC, repeatLimitTargetD] } */;
