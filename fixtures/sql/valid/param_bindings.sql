/* @sqlay
{
  type: query
  id: paramDirectColumnInference
}
*/
SELECT
  c.child_bigint_nn_col AS childBigintNnCol,
  c.varchar_32_nn_col AS childVarchar32NnCol
FROM fixture_child AS c
WHERE c.parent_bigint_nn_col = /* @sqlay { type: param id: parentBigintNnCol } */
  1
  /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: query
  id: paramValueTypeOverride
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE LOWER(p.varchar_320_nn_col) = /* @sqlay { type: param id: lowerVarchar valueType: string } */
  'varchar-320-a'
  /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: query
  id: paramNullableInput
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.datetime_6_col AS datetime6Col
FROM fixture_all_column_type AS p
WHERE p.datetime_6_col = /* @sqlay { type: param id: optionalDatetime nullable: true } */
  '2026-01-02 03:04:05.123456'
  /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: query
  id: paramRepeatedId
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_255_nn_col AS varchar255NnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.varchar_255_nn_col = /* @sqlay { type: param id: searchText } */
  'varchar-255-nn-a'
  /* @sqlay { type: paramEnd } */
  OR p.varchar_320_nn_col = /* @sqlay { type: param id: searchText } */
  'varchar-320-a'
  /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: query
  id: paramInListMarkers
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.char_16_nn_col AS char16NnCol
FROM fixture_all_column_type AS p
WHERE p.char_16_nn_col IN (
  /* @sqlay { type: param id: firstState } */
  'state_a'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: secondState } */
  'state_b'
  /* @sqlay { type: paramEnd } */
)
ORDER BY p.bigint_nn_col;
