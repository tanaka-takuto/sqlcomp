/* @sqlay
{
  type: mutation
  id: mutationMultiTableDelete
}
*/
DELETE c
FROM fixture_child AS c
JOIN fixture_all_column_type AS p ON p.bigint_nn_col = c.parent_bigint_nn_col
WHERE p.bigint_nn_col = 1;
