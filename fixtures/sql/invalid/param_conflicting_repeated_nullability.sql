/* @sqlay
{
  type: query
  id: paramConflictingRepeatedNullability
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.varchar_320_nn_col = /* @sqlay { type: param id: sameText } */
  'varchar-320-a'
  /* @sqlay { type: paramEnd } */
  OR p.varchar_255_nn_col = /* @sqlay { type: param id: sameText nullable: true } */
  'varchar-255-a'
  /* @sqlay { type: paramEnd } */;
