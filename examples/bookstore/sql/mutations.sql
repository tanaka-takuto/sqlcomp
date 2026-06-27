/* @sqlay
{
  type: mutation
  id: createOrder
}
*/
INSERT INTO bookstore_orders (
  customer_id,
  order_number,
  status,
  currency,
  placed_at,
  paid_at,
  shipped_at,
  shipping_method,
  gift_message
) VALUES (
  /* @sqlay { type: param id: customerId } */
  1000
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: orderNumber } */
  'BK-2000'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: status } */
  'draft'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: currency } */
  'USD'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: placedAt } */
  '2026-04-20 12:00:00.000000'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: paidAt nullable: true } */
  '2026-04-20 12:01:00.000000'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: shippedAt nullable: true } */
  '2026-04-21 08:00:00'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: shippingMethod nullable: true } */
  'priority'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: giftMessage nullable: true } */
  'Leave at the side door'
  /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: query
  id: findOrderById
  cardinality: one
}
*/
SELECT
  o.id AS orderId,
  o.order_number AS orderNumber,
  o.status AS orderStatus,
  o.currency AS currency,
  o.placed_at AS placedAt,
  o.paid_at AS paidAt,
  o.shipped_at AS shippedAt,
  o.shipping_method AS shippingMethod,
  o.gift_message AS giftMessage
FROM bookstore_orders AS o
WHERE o.id = /* @sqlay { type: param id: orderId } */
  5000
  /* @sqlay { type: paramEnd } */
LIMIT 1;

/* @sqlay
{
  type: query
  id: findOrderByNumber
  cardinality: one
}
*/
SELECT
  o.id AS orderId,
  o.order_number AS orderNumber,
  o.status AS orderStatus,
  o.currency AS currency,
  o.placed_at AS placedAt,
  o.paid_at AS paidAt,
  o.shipped_at AS shippedAt,
  o.shipping_method AS shippingMethod,
  o.gift_message AS giftMessage
FROM bookstore_orders AS o
WHERE o.order_number = /* @sqlay { type: param id: orderNumber } */
  'BK-1000'
  /* @sqlay { type: paramEnd } */
LIMIT 1;

/* @sqlay
{
  type: mutation
  id: markOrderPaid
}
*/
UPDATE bookstore_orders AS o
SET
  o.status = 'paid',
  o.paid_at = /* @sqlay { type: param id: paidAt } */
    '2026-04-20 12:01:00.000000'
    /* @sqlay { type: paramEnd } */
WHERE o.order_number = /* @sqlay { type: param id: orderNumber } */
  'BK-2000'
  /* @sqlay { type: paramEnd } */
  AND o.status = 'draft'
LIMIT 1;

/* @sqlay
{
  type: mutation
  id: deleteUnapprovedReview
}
*/
DELETE FROM bookstore_reviews AS r
WHERE r.id = /* @sqlay { type: param id: reviewId } */
  7003
  /* @sqlay { type: paramEnd } */
  AND r.approved = 0
LIMIT 1;

/* @sqlay
{
  type: mutation
  id: createOrderItems
}
*/
INSERT INTO bookstore_order_items (
  order_id,
  book_id,
  quantity,
  unit_price,
  discount_amount
) VALUES (
  /* @sqlay { type: param id: orderId } */
  5000
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: firstBookId } */
  100
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: firstQuantity } */
  1
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: firstUnitPrice } */
  16.99
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: firstDiscountAmount nullable: true } */
  0.00
  /* @sqlay { type: paramEnd } */
), (
  /* @sqlay { type: param id: orderId } */
  5000
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: secondBookId } */
  102
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: secondQuantity } */
  1
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: secondUnitPrice } */
  18.00
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: secondDiscountAmount nullable: true } */
  2.00
  /* @sqlay { type: paramEnd } */
);

/* @sqlay
{
  type: mutation
  id: upsertOrderStatus
}
*/
INSERT INTO bookstore_orders (
  customer_id,
  order_number,
  status,
  currency,
  placed_at
) VALUES (
  /* @sqlay { type: param id: customerId } */
  1000
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: orderNumber } */
  'BK-2001'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: initialStatus } */
  'draft'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: currency } */
  'USD'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: placedAt } */
  '2026-04-20 12:00:00.000000'
  /* @sqlay { type: paramEnd } */
) ON DUPLICATE KEY UPDATE
  status = /* @sqlay { type: param id: nextStatus } */
    'paid'
    /* @sqlay { type: paramEnd } */,
  paid_at = /* @sqlay { type: param id: paidAt nullable: true } */
    '2026-04-20 12:01:00.000000'
    /* @sqlay { type: paramEnd } */;

/* @sqlay
{
  type: mutation
  id: replaceCategory
}
*/
REPLACE INTO bookstore_categories (
  id,
  slug,
  display_name
) VALUES (
  /* @sqlay { type: param id: categoryId } */
  13
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: slug } */
  'staff-picks'
  /* @sqlay { type: paramEnd } */,
  /* @sqlay { type: param id: displayName } */
  'Staff Picks'
  /* @sqlay { type: paramEnd } */
);
