/* @sqlay
{
  type: query
  id: directParamRepeatIdCollision
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE p.varchar_320_nn_col = /* @sqlay { type: param id: filter } */
  'varchar-320-a'
  /* @sqlay { type: paramEnd } */
  AND p.bigint_nn_col IN (
    /* @sqlay { type: repeat id: filter separator: "," } */
    /* @sqlay { type: param id: id valueType: int64 } */
    1
    /* @sqlay { type: paramEnd } */
    /* @sqlay { type: repeatEnd } */
  );
