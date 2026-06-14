/* @sqlcomp
{
  type: query
  id: metadataDirectColumns
}
*/
SELECT
  id AS userId,
  display_name AS displayName,
  nickname AS nickname,
  bio AS bio,
  email AS email,
  status AS status,
  login_count AS loginCount,
  reputation AS reputation,
  account_balance AS accountBalance,
  ratio_float AS ratioFloat,
  score_double AS scoreDouble,
  avatar_bytes AS avatarBytes,
  profile_blob AS profileBlob,
  birth_date AS birthDate,
  created_at AS createdAt,
  last_seen_at AS lastSeenAt,
  active AS active,
  settings AS settings
FROM sqlcomp_metadata_users;

/* @sqlcomp
{
  type: query
  id: metadataJoinColumns
}
*/
SELECT
  u.id AS userId,
  u.display_name AS displayName,
  o.id AS orderId,
  o.order_number AS orderNumber,
  o.total AS orderTotal,
  o.tax AS orderTax,
  o.paid_at AS paidAt,
  o.shipped_at AS shippedAt,
  o.delivery_window AS deliveryWindow,
  o.receipt AS receipt
FROM sqlcomp_metadata_users AS u
INNER JOIN sqlcomp_metadata_orders AS o
  ON o.user_id = u.id;

/* @sqlcomp
{
  type: query
  id: metadataLeftJoinColumns
}
*/
SELECT
  u.id AS userId,
  u.display_name AS displayName,
  o.order_number AS orderNumber,
  o.total AS orderTotal,
  o.paid_at AS paidAt,
  o.shipped_at AS shippedAt,
  o.delivery_window AS deliveryWindow,
  o.receipt AS receipt
FROM sqlcomp_metadata_users AS u
LEFT JOIN sqlcomp_metadata_orders AS o
  ON o.user_id = u.id;

/* @sqlcomp
{
  type: query
  id: metadataExpressions
}
*/
SELECT
  u.id + 1 AS nextUserId,
  CONCAT(u.display_name, ':', u.email) AS userLabel,
  COALESCE(u.nickname, u.display_name) AS publicName,
  u.account_balance + o.total AS combinedDecimal,
  o.tax * 2 AS doubledTax,
  u.last_seen_at IS NULL AS missingLastSeen,
  JSON_EXTRACT(u.settings, '$.tier') AS settingsTier
FROM sqlcomp_metadata_users AS u
LEFT JOIN sqlcomp_metadata_orders AS o
  ON o.user_id = u.id;

/* @sqlcomp
{
  type: query
  id: metadataAggregateExpressions
}
*/
SELECT
  u.active AS active,
  COUNT(*) AS userCount,
  SUM(u.login_count) AS totalLoginCount,
  CASE
    WHEN u.active = 1 THEN 'enabled'
    ELSE 'disabled'
  END AS activeState
FROM sqlcomp_metadata_users AS u
GROUP BY u.active;

/* @sqlcomp
{
  type: query
  id: metadataSingleUser
  cardinality: one
}
*/
SELECT
  id AS userId,
  display_name AS displayName,
  nickname AS nickname
FROM sqlcomp_metadata_users
WHERE id = 1
LIMIT 1;
