/* @sqlay
{
  type: query
  id: listAvailableBooks
}
*/
SELECT
  b.id AS bookId,
  b.isbn AS isbn,
  b.title AS title,
  a.display_name AS authorName,
  b.format AS format,
  b.price AS price,
  b.stock_quantity AS stockQuantity,
  COALESCE(ROUND(AVG(r.rating), 2), 0) AS averageRating,
  COUNT(r.id) AS reviewCount,
  CASE
    WHEN b.stock_quantity = 0 THEN 'sold_out'
    WHEN b.stock_quantity <= b.reorder_level THEN 'low_stock'
    ELSE 'available'
  END AS availability
FROM bookstore_books AS b
INNER JOIN bookstore_authors AS a
  ON a.id = b.author_id
LEFT JOIN bookstore_reviews AS r
  ON r.book_id = b.id
  AND r.approved = 1
WHERE b.stock_quantity > 0
/* @sqlay { type: slot id: discoveryFilter targets: [staffPicksOnly, byBookFormat, byBookIds] } */
GROUP BY
  b.id,
  b.isbn,
  b.title,
  a.display_name,
  b.format,
  b.price,
  b.stock_quantity,
  b.reorder_level
ORDER BY b.title;

/* @sqlay
{
  type: query
  id: findBookDetail
  cardinality: one
}
*/
SELECT
  b.id AS bookId,
  b.isbn AS isbn,
  b.title AS title,
  b.description AS description,
  a.display_name AS authorName,
  a.country_code AS authorCountryCode,
  b.format AS format,
  b.price AS price,
  b.published_on AS publishedOn,
  CAST(JSON_UNQUOTE(JSON_EXTRACT(b.metadata, '$.series')) AS CHAR(255)) AS seriesName,
  (
    SELECT MIN(c.display_name)
    FROM bookstore_book_categories AS bc
    INNER JOIN bookstore_categories AS c
      ON c.id = bc.category_id
    WHERE bc.book_id = b.id
  ) AS primaryCategory,
  (
    SELECT COUNT(*)
    FROM bookstore_reviews AS counted_reviews
    WHERE counted_reviews.book_id = b.id
      AND counted_reviews.approved = 1
  ) AS approvedReviewCount,
  (
    SELECT AVG(averaged_reviews.rating)
    FROM bookstore_reviews AS averaged_reviews
    WHERE averaged_reviews.book_id = b.id
      AND averaged_reviews.approved = 1
  ) AS averageRating
FROM bookstore_books AS b
INNER JOIN bookstore_authors AS a
  ON a.id = b.author_id
WHERE b.isbn = /* @sqlay { type: param id: isbn } */
  '9780441478125'
  /* @sqlay { type: paramEnd } */
LIMIT 1;

/* @sqlay
{
  type: query
  id: listBooksNeedingRestock
}
*/
SELECT
  b.id AS bookId,
  b.title AS title,
  b.stock_quantity AS stockQuantity,
  b.reorder_level AS reorderLevel,
  b.reorder_level - b.stock_quantity AS unitsBelowTarget,
  CASE
    WHEN b.stock_quantity = 0 THEN 'out'
    ELSE 'low'
  END AS restockState
FROM bookstore_books AS b
WHERE b.stock_quantity <= b.reorder_level
ORDER BY unitsBelowTarget DESC, b.title;

/* @sqlay
{
  type: query
  id: listTopRatedBooks
}
*/
SELECT
  b.id AS bookId,
  b.title AS title,
  a.display_name AS authorName,
  COUNT(r.id) AS reviewCount,
  AVG(r.rating) AS averageRating
FROM bookstore_books AS b
INNER JOIN bookstore_authors AS a
  ON a.id = b.author_id
INNER JOIN bookstore_reviews AS r
  ON r.book_id = b.id
  AND r.approved = 1
WHERE 1 = 1
/* @sqlay { type: slot id: discoveryFilter targets: [staffPicksOnly, byBookFormat, byBookIds] } */
GROUP BY
  b.id,
  b.title,
  a.display_name
HAVING COUNT(r.id) >= 1
ORDER BY averageRating DESC, reviewCount DESC, b.title
LIMIT 10;
