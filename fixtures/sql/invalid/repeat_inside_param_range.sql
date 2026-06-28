/* @sqlay
{
  type: query
  id: repeatInsideParamRange
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.varchar_320_nn_col = /* @sqlay { type: param id: textValue valueType: string } */
  COALESCE(
    /* @sqlay { type: repeat id: values separator: "," } */
    'varchar-320-a'
    /* @sqlay { type: repeatEnd } */
  )
  /* @sqlay { type: paramEnd } */;
