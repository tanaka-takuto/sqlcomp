/* @sqlay
{
  type: mutation
  id: mutationMultipleStatements
}
*/
UPDATE fixture_child
SET varchar_32_nn_col = 'updated'
WHERE child_bigint_nn_col = 100;

DELETE FROM fixture_child
WHERE child_bigint_nn_col = 100;
