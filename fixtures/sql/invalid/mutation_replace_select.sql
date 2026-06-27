/* @sqlay
{
  type: mutation
  id: mutationReplaceSelect
}
*/
REPLACE INTO fixture_child (
  child_bigint_nn_col,
  parent_bigint_nn_col,
  varchar_32_nn_col,
  decimal_12_2_nn_col
)
SELECT
  child_bigint_nn_col + 1000,
  parent_bigint_nn_col,
  varchar_32_nn_col,
  decimal_12_2_nn_col
FROM fixture_child;
