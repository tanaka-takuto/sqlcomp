/* @sqlay
{
  type: mutation
  id: mutationUnsupportedInferenceContext
}
*/
UPDATE fixture_child AS c
SET c.double_col = COALESCE(c.double_col, 0) + /* @sqlay { type: param id: adjustment } */
  1.25
  /* @sqlay { type: paramEnd } */
WHERE c.child_bigint_nn_col = /* @sqlay { type: param id: childId } */
  100
  /* @sqlay { type: paramEnd } */;
