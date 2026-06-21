/* @sqlay
{
  type: fragment
  id: extraResultColumn
}
*/
  , p.varchar_320_nn_col AS varchar320NnCol

/* @sqlay
{
  type: query
  id: slotVariantRowShapeMismatch
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
/* @sqlay { type: slot id: shape targets: [extraResultColumn] } */
FROM fixture_all_column_type AS p
WHERE p.bigint_nn_col > 0;
