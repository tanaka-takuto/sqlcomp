/* @sqlay
{
  type: fragment
  id: slotRuntimeActiveOnly
}
*/
  AND p.bool_nn_col = TRUE

/* @sqlay
{
  type: fragment
  id: slotRuntimeByVarchar
}
*/
  AND p.varchar_320_nn_col = /* @sqlay { type: param id: varcharFilter } */
    'varchar-320-a'
    /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: fragment
  id: slotRuntimeByChildAmount
}
*/
  AND EXISTS (
    SELECT 1
    FROM fixture_child AS c
    WHERE c.parent_bigint_nn_col = p.bigint_nn_col
      AND c.decimal_12_2_nn_col >= /* @sqlay { type: param id: minAmount valueType: decimal } */
        10.00
        /* @sqlay { type: paramEnd } */
  )

/* @sqlay
{
  type: query
  id: slotRuntimeSearch
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col >= /* @sqlay { type: param id: minId } */
  1
  /* @sqlay { type: paramEnd } */
/* @sqlay { type: slot id: filter targets: [slotRuntimeActiveOnly, slotRuntimeByVarchar, slotRuntimeByChildAmount] } */
  AND p.char_16_nn_col = /* @sqlay { type: param id: state } */
    'state_a'
    /* @sqlay { type: paramEnd } */
ORDER BY p.bigint_nn_col;

/* @sqlay
{
  type: query
  id: slotRuntimeOptionalFilter
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlay { type: slot id: filter targets: [slotRuntimeActiveOnly, slotRuntimeByVarchar, slotRuntimeByChildAmount] } */
ORDER BY p.bigint_nn_col;
