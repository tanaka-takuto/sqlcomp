/* @sqlcomp
{
  type: fragment
  id: fragmentParamSamplePlaceholder
}
*/
  AND p.bigint_nn_col = /* @sqlcomp { type: param id: bigintValue } */
    ?
    /* @sqlcomp { type: paramEnd } */
