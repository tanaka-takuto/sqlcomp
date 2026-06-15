/* @sqlcomp
{
  type: query
  id: listCustomerOrders
}
*/
SELECT
  c.id AS customerId,
  c.full_name AS customerName,
  c.loyalty_tier AS loyaltyTier,
  o.order_number AS orderNumber,
  o.status AS orderStatus,
  o.currency AS currency,
  o.placed_at AS placedAt,
  o.shipped_at AS shippedAt,
  COUNT(oi.id) AS lineCount,
  SUM(oi.quantity) AS totalQuantity,
  SUM(oi.quantity * oi.unit_price - COALESCE(oi.discount_amount, 0)) AS orderTotal,
  CASE
    WHEN o.shipped_at IS NULL THEN 'not_shipped'
    ELSE 'shipped'
  END AS shippingState
FROM bookstore_orders AS o
INNER JOIN bookstore_customers AS c
  ON c.id = o.customer_id
INNER JOIN bookstore_order_items AS oi
  ON oi.order_id = o.id
GROUP BY
  c.id,
  c.full_name,
  c.loyalty_tier,
  o.order_number,
  o.status,
  o.currency,
  o.placed_at,
  o.shipped_at
ORDER BY o.placed_at DESC;

/* @sqlcomp
{
  type: query
  id: findLatestOrderForCustomer
}
*/
SELECT
  o.id AS orderId,
  o.order_number AS orderNumber,
  o.status AS orderStatus,
  o.placed_at AS placedAt,
  o.paid_at AS paidAt,
  o.shipped_at AS shippedAt,
  o.gift_message AS giftMessage,
  SUM(oi.quantity * oi.unit_price - COALESCE(oi.discount_amount, 0)) AS orderTotal
FROM bookstore_orders AS o
INNER JOIN bookstore_customers AS c
  ON c.id = o.customer_id
INNER JOIN bookstore_order_items AS oi
  ON oi.order_id = o.id
WHERE c.email = 'river@example.test'
GROUP BY
  o.id,
  o.order_number,
  o.status,
  o.placed_at,
  o.paid_at,
  o.shipped_at,
  o.gift_message
ORDER BY o.placed_at DESC
LIMIT 1;

/* @sqlcomp
{
  type: query
  id: listUnreviewedPurchases
}
*/
SELECT
  c.id AS customerId,
  c.full_name AS customerName,
  b.id AS bookId,
  b.title AS bookTitle,
  o.order_number AS orderNumber,
  oi.id AS orderItemId,
  o.placed_at AS purchasedAt
FROM bookstore_order_items AS oi
INNER JOIN bookstore_orders AS o
  ON o.id = oi.order_id
INNER JOIN bookstore_customers AS c
  ON c.id = o.customer_id
INNER JOIN bookstore_books AS b
  ON b.id = oi.book_id
LEFT JOIN bookstore_reviews AS r
  ON r.order_item_id = oi.id
WHERE r.id IS NULL
  AND o.status IN ('paid', 'shipped', 'delivered')
ORDER BY o.placed_at DESC, b.title;

/* @sqlcomp
{
  type: query
  id: listMonthlySales
}
*/
SELECT
  DATE_FORMAT(o.placed_at, '%Y-%m-01') AS salesMonth,
  COUNT(DISTINCT o.id) AS orderCount,
  SUM(oi.quantity) AS booksSold,
  SUM(oi.quantity * oi.unit_price - COALESCE(oi.discount_amount, 0)) AS grossSales,
  AVG(oi.quantity * oi.unit_price - COALESCE(oi.discount_amount, 0)) AS averageLineTotal
FROM bookstore_orders AS o
INNER JOIN bookstore_order_items AS oi
  ON oi.order_id = o.id
WHERE o.status IN ('paid', 'shipped', 'delivered')
GROUP BY DATE_FORMAT(o.placed_at, '%Y-%m-01')
ORDER BY salesMonth;
