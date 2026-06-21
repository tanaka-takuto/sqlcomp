/* @sqlay
{
  type: fragment
  id: slotFixtureActiveOnly
}
*/
  AND p.bool_nn_col = TRUE

/* @sqlay
{
  type: fragment
  id: slotFixtureByText
}
*/
  AND p.varchar_320_nn_col = /* @sqlay { type: param id: textFilter } */
    'varchar-320-a'
    /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: fragment
  id: slotFixtureByAmount
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
  type: fragment
  id: slotFixtureNullableCreated
}
*/
  AND p.datetime_6_col >= /* @sqlay { type: param id: createdAfter nullable: true } */
    '2026-01-02 03:04:05.123456'
    /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: fragment
  id: slotFixtureEqualsValue
}
*/
  = /* @sqlay { type: param id: value } */
    'varchar-320-a'
    /* @sqlay { type: paramEnd } */ THEN TRUE
  WHEN 1
