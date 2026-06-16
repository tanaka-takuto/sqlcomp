/* @sqlcomp
{
  type: query
  id: paramNestedRanges
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = /* @sqlcomp { type: param id: outerValue } */
  (
    /* @sqlcomp { type: param id: innerValue } */
    1
    /* @sqlcomp { type: paramEnd } */
  )
  /* @sqlcomp { type: paramEnd } */;
