/* @sqlcomp
{
  type: query
  id: paramUnsupportedInferenceContext
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE LOWER(p.varchar_320_nn_col) = /* @sqlcomp { type: param id: lowerVarchar } */
  'varchar-320-a'
  /* @sqlcomp { type: paramEnd } */;
