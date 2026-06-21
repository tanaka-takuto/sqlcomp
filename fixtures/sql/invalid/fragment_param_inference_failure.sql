/* @sqlcomp
{
  type: fragment
  id: lowerTextFilter
}
*/
  AND LOWER(p.varchar_320_nn_col) = /* @sqlcomp { type: param id: lowerText } */
    'varchar-320-a'
    /* @sqlcomp { type: paramEnd } */

/* @sqlcomp
{
  type: query
  id: fragmentParamInferenceFailure
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlcomp { type: slot id: filter targets: [lowerTextFilter] } */;
