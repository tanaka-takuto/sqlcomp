/* @sqlay
{
  type: query
  id: nonSelect
}
*/
INSERT INTO fixture_all_column_type (
  bigint_nn_col,
  varchar_255_nn_col,
  varchar_320_nn_col,
  char_16_nn_col,
  int_nn_col,
  int_unsigned_nn_col,
  decimal_18_4_nn_col,
  double_nn_col,
  datetime_6_nn_col,
  tinyint_1_nn_col
)
VALUES (
  1,
  'varchar-255-nn-a',
  'varchar-320-a',
  'state_a',
  1,
  1,
  1.0000,
  1.0,
  '2026-01-02 03:04:05.123456',
  1
);
