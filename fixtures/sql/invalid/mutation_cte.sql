/* @sqlay
{
  type: mutation
  id: mutationCte
}
*/
WITH target_rows AS (
  SELECT child_bigint_nn_col
  FROM fixture_child
)
UPDATE fixture_child
SET varchar_32_nn_col = 'updated'
WHERE child_bigint_nn_col IN (SELECT child_bigint_nn_col FROM target_rows);
