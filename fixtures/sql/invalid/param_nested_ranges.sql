/* @sqlay
{
  type: query
  id: paramNestedRanges
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = /* @sqlay { type: param id: outerValue } */
  (
    /* @sqlay { type: param id: innerValue } */
    1
    /* @sqlay { type: paramEnd } */
  )
  /* @sqlay { type: paramEnd } */;
