/* @sqlay
{
  type: mutation
  id: mutationInsertValues
}
*/
INSERT INTO fixture_all_column_type (
  bigint_nn_col,
  int_nn_col,
  varchar_320_nn_col,
  bool_nn_col,
  datetime_6_nn_col
) VALUES (
  /* @sqlay { type: param id: bigintId } */
  10
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: intValue } */
  7
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: textValue } */
  'varchar-320-new'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: activeValue } */
  TRUE
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: createdAt } */
  '2026-06-21 10:11:12.123456'
  /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: mutation
  id: mutationInsertSet
}
*/
INSERT INTO fixture_child SET
  child_bigint_nn_col = /* @sqlay { type: param id: childId } */
    200
    /* @sqlay { type: paramEnd } */,
  parent_bigint_nn_col = /* @sqlay { type: param id: parentId } */
    1
    /* @sqlay { type: paramEnd } */,
  varchar_32_nn_col = /* @sqlay { type: param id: childLabel } */
    'child-new'
    /* @sqlay { type: paramEnd } */,
  decimal_12_2_nn_col = /* @sqlay { type: param id: childAmount } */
    25.50
    /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: mutation
  id: mutationUpsertValues
}
*/
INSERT INTO fixture_child (
  child_bigint_nn_col,
  parent_bigint_nn_col,
  varchar_32_nn_col,
  decimal_12_2_nn_col
) VALUES (
  /* @sqlay { type: param id: childId } */
  201
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: parentId } */
  1
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childLabel } */
  'child-upsert'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childAmount } */
  30.75
  /* @sqlay { type: paramEnd } */
) ON DUPLICATE KEY UPDATE
  varchar_32_nn_col = /* @sqlay { type: param id: updatedLabel } */
    'child-updated'
    /* @sqlay { type: paramEnd } */,
  decimal_12_2_nn_col = /* @sqlay { type: param id: updatedAmount } */
    31.25
    /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: mutation
  id: mutationReplaceValues
}
*/
REPLACE INTO fixture_child (
  child_bigint_nn_col,
  parent_bigint_nn_col,
  varchar_32_nn_col,
  decimal_12_2_nn_col
) VALUES (
  /* @sqlay { type: param id: childId } */
  202
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: parentId } */
  2
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childLabel } */
  'child-replace'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childAmount } */
  40.00
  /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: mutation
  id: mutationReplaceSet
}
*/
REPLACE INTO fixture_child SET
  child_bigint_nn_col = /* @sqlay { type: param id: childId } */
    203
    /* @sqlay { type: paramEnd } */,
  parent_bigint_nn_col = /* @sqlay { type: param id: parentId } */
    2
    /* @sqlay { type: paramEnd } */,
  varchar_32_nn_col = /* @sqlay { type: param id: childLabel } */
    'child-replace-set'
    /* @sqlay { type: paramEnd } */,
  decimal_12_2_nn_col = /* @sqlay { type: param id: childAmount } */
    41.00
    /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: mutation
  id: mutationUpdateAliasLimited
}
*/
UPDATE fixture_all_column_type AS p
SET p.varchar_320_nn_col = /* @sqlay { type: param id: textValue } */
  'varchar-320-updated'
  /* @sqlay { type: paramEnd } */
WHERE p.bigint_nn_col = /* @sqlay { type: param id: bigintId } */
  1
  /* @sqlay { type: paramEnd } */
ORDER BY p.bigint_nn_col
LIMIT 1;

/* @sqlay
{
  type: mutation
  id: mutationDeleteAliasLimited
}
*/
DELETE FROM fixture_child AS c
WHERE c.parent_bigint_nn_col = /* @sqlay { type: param id: parentId } */
  2
  /* @sqlay { type: paramEnd } */
ORDER BY c.child_bigint_nn_col
LIMIT 1;

/* @sqlay
{
  type: mutation
  id: mutationValueTypeOverride
}
*/
UPDATE fixture_child AS c
SET c.double_col = COALESCE(c.double_col, 0) + /* @sqlay { type: param id: adjustment valueType: float64 } */
  1.25
  /* @sqlay { type: paramEnd } */
WHERE c.child_bigint_nn_col = /* @sqlay { type: param id: childId } */
  100
  /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: fragment
  id: mutationAssignNullableText
}
*/
,
  p.text_col = /* @sqlay { type: param id: textValue nullable: true } */
    'optional text'
    /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: fragment
  id: mutationAssignDecimal
}
*/
,
  p.decimal_18_4_col = /* @sqlay { type: param id: amount } */
    55.5000
    /* @sqlay { type: paramEnd } */

/* @sqlay
{
  type: mutation
  id: mutationSlotAssignment
}
*/
UPDATE fixture_all_column_type AS p
SET p.varchar_320_nn_col = /* @sqlay { type: param id: textValue } */
  'slot-base'
  /* @sqlay { type: paramEnd } */
/* @sqlay { type: slot id: assignment targets: [mutationAssignNullableText, mutationAssignDecimal] } */
WHERE p.bigint_nn_col = /* @sqlay { type: param id: bigintId } */
  1
  /* @sqlay { type: paramEnd } */;
