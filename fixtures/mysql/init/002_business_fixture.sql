USE sqlcomp;

DROP TABLE IF EXISTS sqlcomp_business_order_items;
DROP TABLE IF EXISTS sqlcomp_business_orders;
DROP TABLE IF EXISTS sqlcomp_business_customers;

CREATE TABLE sqlcomp_business_customers (
  id BIGINT NOT NULL PRIMARY KEY,
  tenant_key VARCHAR(64) NOT NULL,
  external_ref VARCHAR(64) NULL,
  email VARCHAR(320) NOT NULL,
  full_name VARCHAR(255) NOT NULL,
  phone VARCHAR(32) NULL,
  lifecycle_status ENUM('lead', 'active', 'paused', 'churned') NOT NULL,
  lifetime_value DECIMAL(14, 2) NOT NULL,
  marketing_opt_in TINYINT(1) NOT NULL,
  first_seen_on DATE NOT NULL,
  last_contacted_at DATETIME(6) NULL,
  internal_notes TEXT NULL,
  tags JSON NULL,
  UNIQUE KEY uq_sqlcomp_business_customers_email (tenant_key, email),
  KEY idx_sqlcomp_business_customers_status (tenant_key, lifecycle_status)
);

CREATE TABLE sqlcomp_business_orders (
  id BIGINT NOT NULL PRIMARY KEY,
  tenant_key VARCHAR(64) NOT NULL,
  customer_id BIGINT NOT NULL,
  order_number VARCHAR(32) NOT NULL,
  status ENUM('draft', 'placed', 'paid', 'fulfilled', 'cancelled', 'refunded') NOT NULL,
  currency CHAR(3) NOT NULL,
  subtotal DECIMAL(12, 2) NOT NULL,
  discount_total DECIMAL(12, 2) NOT NULL,
  tax_total DECIMAL(12, 2) NOT NULL,
  grand_total DECIMAL(12, 2) NOT NULL,
  risk_score DOUBLE NULL,
  placed_at DATETIME(6) NOT NULL,
  paid_at DATETIME(6) NULL,
  shipped_at TIMESTAMP NULL DEFAULT NULL,
  delivery_window TIME NULL,
  source_channel VARCHAR(32) NOT NULL,
  payment_reference VARBINARY(32) NULL,
  notes TEXT NULL,
  attributes JSON NULL,
  UNIQUE KEY uq_sqlcomp_business_orders_number (tenant_key, order_number),
  KEY idx_sqlcomp_business_orders_customer (tenant_key, customer_id, placed_at),
  CONSTRAINT fk_sqlcomp_business_orders_customer
    FOREIGN KEY (customer_id) REFERENCES sqlcomp_business_customers (id)
);

CREATE TABLE sqlcomp_business_order_items (
  id BIGINT NOT NULL PRIMARY KEY,
  tenant_key VARCHAR(64) NOT NULL,
  order_id BIGINT NOT NULL,
  line_number SMALLINT UNSIGNED NOT NULL,
  sku VARCHAR(64) NULL,
  description VARCHAR(255) NOT NULL,
  quantity INT UNSIGNED NOT NULL,
  unit_price DECIMAL(12, 2) NOT NULL,
  discount_amount DECIMAL(12, 2) NULL,
  tax_rate DECIMAL(5, 4) NOT NULL,
  fulfilled_quantity INT UNSIGNED NOT NULL,
  item_metadata JSON NULL,
  UNIQUE KEY uq_sqlcomp_business_order_items_line (order_id, line_number),
  KEY idx_sqlcomp_business_order_items_sku (tenant_key, sku),
  CONSTRAINT fk_sqlcomp_business_order_items_order
    FOREIGN KEY (order_id) REFERENCES sqlcomp_business_orders (id)
);

INSERT INTO sqlcomp_business_customers (
  id,
  tenant_key,
  external_ref,
  email,
  full_name,
  phone,
  lifecycle_status,
  lifetime_value,
  marketing_opt_in,
  first_seen_on,
  last_contacted_at,
  internal_notes,
  tags
) VALUES
  (
    10,
    'acme',
    'crm-100',
    'pat@example.test',
    'Pat Buyer',
    '+1-555-0100',
    'active',
    2450.75,
    1,
    '2024-04-01',
    '2026-03-10 11:12:13.000000',
    'Prefers invoice by email.',
    JSON_ARRAY('vip', 'newsletter')
  ),
  (
    11,
    'acme',
    NULL,
    'lee@example.test',
    'Lee Trial',
    NULL,
    'lead',
    0.00,
    0,
    '2026-01-15',
    NULL,
    NULL,
    JSON_ARRAY('trial')
  ),
  (
    20,
    'northwind',
    'erp-200',
    'sam@example.test',
    'Sam Operator',
    '+44-20-0000-0000',
    'paused',
    120.00,
    1,
    '2025-07-20',
    '2026-02-01 09:00:00.000000',
    'Has no local order rows for left join smoke tests.',
    NULL
  );

INSERT INTO sqlcomp_business_orders (
  id,
  tenant_key,
  customer_id,
  order_number,
  status,
  currency,
  subtotal,
  discount_total,
  tax_total,
  grand_total,
  risk_score,
  placed_at,
  paid_at,
  shipped_at,
  delivery_window,
  source_channel,
  payment_reference,
  notes,
  attributes
) VALUES
  (
    5000,
    'acme',
    10,
    'ACME-1000',
    'paid',
    'USD',
    138.98,
    10.00,
    8.72,
    137.70,
    0.12,
    '2026-03-04 10:11:12.123456',
    '2026-03-04 10:12:00.000000',
    '2026-03-05 08:00:00',
    '02:30:00',
    'web',
    X'00112233445566778899AABBCCDDEEFF',
    'Leave at reception.',
    JSON_OBJECT('coupon', 'WELCOME10', 'gift', true)
  ),
  (
    5001,
    'acme',
    11,
    'ACME-1001',
    'draft',
    'USD',
    19.99,
    0.00,
    0.00,
    19.99,
    NULL,
    '2026-03-06 14:00:00.000000',
    NULL,
    NULL,
    NULL,
    'sales',
    NULL,
    NULL,
    JSON_OBJECT('quoteId', 'Q-55')
  );

INSERT INTO sqlcomp_business_order_items (
  id,
  tenant_key,
  order_id,
  line_number,
  sku,
  description,
  quantity,
  unit_price,
  discount_amount,
  tax_rate,
  fulfilled_quantity,
  item_metadata
) VALUES
  (
    9000,
    'acme',
    5000,
    1,
    'SKU-WIDGET',
    'Widget',
    2,
    19.99,
    NULL,
    0.0825,
    2,
    JSON_OBJECT('color', 'blue', 'hazmat', false)
  ),
  (
    9001,
    'acme',
    5000,
    2,
    'SKU-SERVICE',
    'Installation Service',
    1,
    99.00,
    10.00,
    0.0825,
    1,
    JSON_OBJECT('durationMinutes', 90)
  ),
  (
    9002,
    'acme',
    5001,
    1,
    'SKU-WIDGET',
    'Widget',
    1,
    19.99,
    NULL,
    0.0000,
    0,
    NULL
  );
