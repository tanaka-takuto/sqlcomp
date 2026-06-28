/* @sqlay
{
  type: query
  id: repeatParamInferenceFailure
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE LOWER(p.varchar_320_nn_col) IN (
  /* @sqlay { type: repeat id: values separator: "," } */
  /* @sqlay { type: param id: value } */
  'varchar-320-a'
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
);
