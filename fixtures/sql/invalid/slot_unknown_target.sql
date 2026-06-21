/* @sqlcomp
{
  type: query
  id: slotUnknownTarget
}
*/
SELECT
  p.bigint_nn_col AS bigintNnCol
FROM fixture_all_column_type AS p
WHERE 1 = 1
/* @sqlcomp { type: slot id: filter targets: [missingFilter] } */;
