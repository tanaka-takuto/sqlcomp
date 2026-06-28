/* @sqlay
{
  type: query
  id: repeatDynamicInList
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.char_16_nn_col AS char16NnCol
FROM fixture_all_column_type AS p
WHERE p.char_16_nn_col IN (
  /* @sqlay { type: repeat id: states separator: ", " } */
  /* @sqlay { type: param id: state } */
  'state_a'
  /* @sqlay { type: paramEnd } */
  /* @sqlay { type: repeatEnd } */
)
ORDER BY p.bigint_nn_col;

/* @sqlay
{
  type: mutation
  id: repeatBulkInsertRows
}
*/
INSERT INTO fixture_child (
  child_bigint_nn_col,
  parent_bigint_nn_col,
  varchar_32_nn_col,
  decimal_12_2_nn_col
) VALUES
/* @sqlay { type: repeat id: rows separator: "," } */
(
  /* @sqlay { type: param id: childId } */
  300
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: parentId } */
  1
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childLabel } */
  'child-repeat'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: childAmount } */
  42.50
  /* @sqlay { type: paramEnd } */
)
/* @sqlay { type: repeatEnd } */;

/* @sqlay
{
  type: query
  id: repeatRepeatedIdEmission
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE EXISTS (
  SELECT 1
  FROM fixture_child AS c
  WHERE c.parent_bigint_nn_col = p.bigint_nn_col
    AND (
      /* @sqlay { type: repeat id: childPairs separator: " OR " } */
      (
        c.child_bigint_nn_col = /* @sqlay { type: param id: childId valueType: int64 } */
          100
          /* @sqlay { type: paramEnd } */
        AND c.parent_bigint_nn_col = /* @sqlay { type: param id: parentId valueType: int64 } */
          1
          /* @sqlay { type: paramEnd } */
        AND c.parent_bigint_nn_col >= /* @sqlay { type: param id: parentId valueType: int64 } */
          1
          /* @sqlay { type: paramEnd } */
      )
      /* @sqlay { type: repeatEnd } */
    )
)
OR (p.bigint_nn_col, p.bigint_nn_col) IN (
  /* @sqlay { type: repeat id: childPairs separator: ", " } */
  (
    /* @sqlay { type: param id: parentId valueType: int64 } */
    1
    /* @sqlay { type: paramEnd } */,
    /* @sqlay { type: param id: childId valueType: int64 } */
    100
    /* @sqlay { type: paramEnd } */
  )
  /* @sqlay { type: repeatEnd } */
)
ORDER BY p.bigint_nn_col;

/* @sqlay
{
  type: fragment
  id: repeatFixtureChildIds
}
*/
AND EXISTS (
  SELECT 1
  FROM fixture_child AS c
  WHERE c.parent_bigint_nn_col = p.bigint_nn_col
    AND c.child_bigint_nn_col IN (
      /* @sqlay { type: repeat id: childIds separator: ", " } */
      /* @sqlay { type: param id: childId valueType: int64 } */
      100
      /* @sqlay { type: paramEnd } */
      /* @sqlay { type: repeatEnd } */
    )
)

/* @sqlay
{
  type: query
  id: repeatFragmentSlotSearch
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol,
  p.varchar_320_nn_col AS varchar320NnCol
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col IS NOT NULL
/* @sqlay { type: slot id: requiredChildFilter targets: [repeatFixtureChildIds] } */
/* @sqlay { type: slot id: optionalChildFilter targets: [repeatFixtureChildIds] } */
ORDER BY p.bigint_nn_col;
