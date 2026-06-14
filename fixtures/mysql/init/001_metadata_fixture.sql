CREATE DATABASE IF NOT EXISTS sqlcomp
  CHARACTER SET utf8mb4
  COLLATE utf8mb4_0900_ai_ci;

USE sqlcomp;

DROP TABLE IF EXISTS sqlcomp_metadata_orders;
DROP TABLE IF EXISTS sqlcomp_metadata_users;

CREATE TABLE sqlcomp_metadata_users (
  id BIGINT NOT NULL PRIMARY KEY,
  display_name VARCHAR(255) NOT NULL,
  nickname VARCHAR(255) NULL,
  bio TEXT NULL,
  email VARCHAR(320) NOT NULL,
  status CHAR(16) NOT NULL,
  login_count INT NOT NULL,
  reputation BIGINT NULL,
  account_balance DECIMAL(18, 4) NOT NULL,
  ratio_float FLOAT NULL,
  score_double DOUBLE NOT NULL,
  avatar_bytes VARBINARY(64) NULL,
  profile_blob BLOB NULL,
  birth_date DATE NULL,
  created_at DATETIME(6) NOT NULL,
  last_seen_at TIMESTAMP NULL DEFAULT NULL,
  active TINYINT(1) NOT NULL,
  settings JSON NULL
);

CREATE TABLE sqlcomp_metadata_orders (
  id BIGINT NOT NULL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  order_number VARCHAR(32) NOT NULL,
  total DECIMAL(12, 2) NOT NULL,
  tax DOUBLE NULL,
  paid_at DATETIME NULL,
  shipped_at TIMESTAMP NULL DEFAULT NULL,
  delivery_window TIME NULL,
  receipt JSON NULL,
  CONSTRAINT fk_sqlcomp_metadata_orders_user
    FOREIGN KEY (user_id) REFERENCES sqlcomp_metadata_users (id)
);

INSERT INTO sqlcomp_metadata_users (
  id,
  display_name,
  nickname,
  bio,
  email,
  status,
  login_count,
  reputation,
  account_balance,
  ratio_float,
  score_double,
  avatar_bytes,
  profile_blob,
  birth_date,
  created_at,
  last_seen_at,
  active,
  settings
) VALUES
  (
    1,
    'Ada Lovelace',
    'ada',
    'First programmer and computing pioneer.',
    'ada@example.test',
    'active',
    7,
    9000000000,
    1234.5600,
    0.75,
    98.125,
    X'01020304',
    X'0A0B0C',
    '1815-12-10',
    '2026-01-02 03:04:05.123456',
    '2026-01-03 04:05:06',
    1,
    JSON_OBJECT('theme', 'dark', 'tier', 'founder')
  ),
  (
    2,
    'Grace Hopper',
    NULL,
    NULL,
    'grace@example.test',
    'invited',
    0,
    NULL,
    0.0000,
    NULL,
    87.5,
    NULL,
    NULL,
    '1906-12-09',
    '2026-02-03 04:05:06.000000',
    NULL,
    0,
    JSON_OBJECT('theme', 'light', 'tier', 'guest')
  ),
  (
    3,
    'No Order User',
    NULL,
    'Seed row without related orders for left join cases.',
    'no-orders@example.test',
    'active',
    1,
    100,
    10.5000,
    1.25,
    75.0,
    X'FF00',
    NULL,
    NULL,
    '2026-04-05 06:07:08.000000',
    NULL,
    1,
    NULL
  );

INSERT INTO sqlcomp_metadata_orders (
  id,
  user_id,
  order_number,
  total,
  tax,
  paid_at,
  shipped_at,
  delivery_window,
  receipt
) VALUES
  (
    100,
    1,
    'A-100',
    49.95,
    4.995,
    '2026-03-04 05:06:07',
    '2026-03-05 06:07:08',
    '02:30:00',
    JSON_OBJECT('currency', 'USD', 'lineCount', 2)
  ),
  (
    101,
    2,
    'G-101',
    12.00,
    NULL,
    NULL,
    NULL,
    NULL,
    NULL
  );
