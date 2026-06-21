/* @sqlay
{
  type: fragment
  id: staffPicksOnly
}
*/
  AND EXISTS (
    SELECT 1
    FROM bookstore_book_categories AS filter_bc
    INNER JOIN bookstore_categories AS filter_c
      ON filter_c.id = filter_bc.category_id
    WHERE filter_bc.book_id = b.id
      AND filter_c.slug = 'staff-picks'
  )

/* @sqlay
{
  type: fragment
  id: byBookFormat
}
*/
  AND b.format = /* @sqlay { type: param id: format } */
    'paperback'
    /* @sqlay { type: paramEnd } */
