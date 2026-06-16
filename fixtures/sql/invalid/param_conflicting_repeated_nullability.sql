/* @sqlcomp
{
  type: query
  id: paramConflictingRepeatedNullability
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.varchar_320_nn_col = /* @sqlcomp { type: param id: sameText } */
  'varchar-320-a'
  /* @sqlcomp { type: paramEnd } */
  OR p.varchar_255_nn_col = /* @sqlcomp { type: param id: sameText nullable: true } */
  'varchar-255-a'
  /* @sqlcomp { type: paramEnd } */;
