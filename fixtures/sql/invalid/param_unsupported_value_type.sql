/* @sqlcomp
{
  type: query
  id: paramUnsupportedValueType
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col = /* @sqlcomp { type: param id: bigintValue valueType: banana } */
  1
  /* @sqlcomp { type: paramEnd } */;
