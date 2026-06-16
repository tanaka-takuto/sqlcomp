/* @sqlcomp
{
  type: query
  id: paramConflictingRepeatedType
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = /* @sqlcomp { type: param id: sameValue } */
  1
  /* @sqlcomp { type: paramEnd } */
  OR p.varchar_320_nn_col = /* @sqlcomp { type: param id: sameValue } */
  'varchar-320-a'
  /* @sqlcomp { type: paramEnd } */;
