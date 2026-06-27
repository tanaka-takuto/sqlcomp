/* @sqlay
{
  type: mutation
  id: mutationRawPlaceholder
}
*/
UPDATE fixture_child
SET varchar_32_nn_col = ?
WHERE child_bigint_nn_col = 100;
