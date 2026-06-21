/* @sqlcomp
{
  type: fragment
  id: equalsValue
}
*/
  = /* @sqlcomp { type: param id: value } */
    'varchar-320-a'
    /* @sqlcomp { type: paramEnd } */ THEN TRUE
  WHEN 1

/* @sqlcomp
{
  type: query
  id: repeatedSlotFragmentParamTypeConflict
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE CASE
  WHEN p.varchar_320_nn_col
/* @sqlcomp { type: slot id: comparator targets: [equalsValue] } */
  IS NULL THEN TRUE
  ELSE TRUE
END
  AND CASE
    WHEN p.bigint_nn_col
/* @sqlcomp { type: slot id: comparator targets: [equalsValue] } */
    IS NULL THEN TRUE
    ELSE TRUE
  END;
