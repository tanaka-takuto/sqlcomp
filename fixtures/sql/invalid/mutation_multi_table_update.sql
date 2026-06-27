/* @sqlay
{
  type: mutation
  id: mutationMultiTableUpdate
}
*/
UPDATE fixture_all_column_type AS p
JOIN fixture_child AS c ON c.parent_bigint_nn_col = p.bigint_nn_col
SET p.varchar_320_nn_col = 'updated'
WHERE c.child_bigint_nn_col = 100;
