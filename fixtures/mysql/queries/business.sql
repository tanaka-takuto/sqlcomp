/* @sqlcomp
{
  type: query
  id: businessOrderSummary
}
*/
SELECT
  c.tenant_key AS tenantKey,
  o.order_number AS orderNumber,
  c.email AS customerEmail,
  c.full_name AS customerName,
  o.status AS orderStatus,
  o.currency AS currency,
  o.grand_total AS grandTotal,
  o.payment_reference AS paymentReference,
  o.notes AS orderNotes,
  (
    SELECT COUNT(*)
    FROM sqlcomp_business_order_items AS counted_items
    WHERE counted_items.order_id = o.id
  ) AS itemCount,
  (
    SELECT SUM(summed_items.quantity)
    FROM sqlcomp_business_order_items AS summed_items
    WHERE summed_items.order_id = o.id
  ) AS totalQuantity,
  CASE
    WHEN o.paid_at IS NULL THEN 'unpaid'
    ELSE 'paid'
  END AS paymentState
FROM sqlcomp_business_orders AS o
INNER JOIN sqlcomp_business_customers AS c
  ON c.id = o.customer_id;

/* @sqlcomp
{
  type: query
  id: businessCustomerProfile
  cardinality: one
}
*/
SELECT
  c.tenant_key AS tenantKey,
  c.id AS customerId,
  c.external_ref AS externalRef,
  c.email AS email,
  c.full_name AS fullName,
  c.phone AS phone,
  c.lifecycle_status AS lifecycleStatus,
  c.lifetime_value AS lifetimeValue,
  c.marketing_opt_in AS marketingOptIn,
  c.first_seen_on AS firstSeenOn,
  c.last_contacted_at AS lastContactedAt,
  c.internal_notes AS internalNotes,
  JSON_UNQUOTE(JSON_EXTRACT(c.tags, '$[0]')) AS firstTag,
  (
    SELECT COUNT(*)
    FROM sqlcomp_business_orders AS counted_orders
    WHERE counted_orders.customer_id = c.id
  ) AS orderCount,
  (
    SELECT MAX(recent_orders.placed_at)
    FROM sqlcomp_business_orders AS recent_orders
    WHERE recent_orders.customer_id = c.id
  ) AS lastOrderAt
FROM sqlcomp_business_customers AS c
WHERE c.id = 10
LIMIT 1;

/* @sqlcomp
{
  type: query
  id: businessCustomerOrderLeftJoin
}
*/
SELECT
  c.id AS customerId,
  c.tenant_key AS tenantKey,
  c.email AS email,
  c.lifecycle_status AS lifecycleStatus,
  o.order_number AS orderNumber,
  o.status AS orderStatus,
  o.grand_total AS grandTotal,
  o.paid_at AS paidAt,
  o.shipped_at AS shippedAt,
  o.delivery_window AS deliveryWindow
FROM sqlcomp_business_customers AS c
LEFT JOIN sqlcomp_business_orders AS o
  ON o.customer_id = c.id;

/* @sqlcomp
{
  type: query
  id: businessLineItemTotals
}
*/
SELECT
  o.order_number AS orderNumber,
  oi.line_number AS lineNumber,
  oi.sku AS sku,
  oi.description AS description,
  oi.quantity AS quantity,
  oi.unit_price AS unitPrice,
  oi.discount_amount AS discountAmount,
  oi.quantity * oi.unit_price - COALESCE(oi.discount_amount, 0) AS lineTotal,
  CASE
    WHEN oi.fulfilled_quantity >= oi.quantity THEN 'fulfilled'
    ELSE 'open'
  END AS fulfillmentState,
  JSON_EXTRACT(oi.item_metadata, '$.hazmat') AS hazmat
FROM sqlcomp_business_order_items AS oi
INNER JOIN sqlcomp_business_orders AS o
  ON o.id = oi.order_id;
