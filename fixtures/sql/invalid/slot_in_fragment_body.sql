/* @sqlcomp
{
  type: fragment
  id: slotInFragmentBody
}
*/
  AND p.bool_nn_col = TRUE
  /* @sqlcomp { type: slot id: nestedFilter targets: [activeOnly] } */
