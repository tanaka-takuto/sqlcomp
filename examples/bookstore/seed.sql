INSERT INTO bookstore_authors (
  id,
  display_name,
  country_code,
  biography
) VALUES
  (1, 'Ursula K. Le Guin', 'US', 'Author of speculative fiction and essays.'),
  (2, 'N. K. Jemisin', 'US', 'Author of award-winning fantasy novels.'),
  (3, 'Octavia E. Butler', 'US', 'Author of science fiction novels.'),
  (4, 'Italo Calvino', 'IT', NULL);

INSERT INTO bookstore_customers (
  id,
  email,
  full_name,
  loyalty_tier,
  joined_on,
  marketing_opt_in,
  last_seen_at
) VALUES
  (1000, 'river@example.test', 'River Reader', 'gold', '2024-01-15', 1, '2026-04-10 09:30:00.000000'),
  (1001, 'morgan@example.test', 'Morgan Pages', 'silver', '2025-06-01', 0, '2026-04-12 18:45:00.000000'),
  (1002, 'casey@example.test', 'Casey Stack', 'standard', '2026-02-20', 1, NULL);

INSERT INTO bookstore_categories (
  id,
  slug,
  display_name
) VALUES
  (10, 'science-fiction', 'Science Fiction'),
  (11, 'fantasy', 'Fantasy'),
  (12, 'literary-fiction', 'Literary Fiction'),
  (13, 'staff-picks', 'Staff Picks');

INSERT INTO bookstore_books (
  id,
  author_id,
  isbn,
  title,
  description,
  format,
  price,
  stock_quantity,
  reorder_level,
  published_on,
  created_at,
  metadata
) VALUES
  (
    100,
    1,
    '9780441478125',
    'The Left Hand of Darkness',
    'A classic novel of diplomacy and identity.',
    'paperback',
    16.99,
    12,
    3,
    '1969-03-01',
    '2026-01-01 10:00:00.000000',
    JSON_OBJECT('shelf', 'A1', 'series', NULL)
  ),
  (
    101,
    1,
    '9780547773705',
    'A Wizard of Earthsea',
    'A coming-of-age fantasy novel.',
    'hardcover',
    22.50,
    2,
    4,
    '1968-11-01',
    '2026-01-02 10:00:00.000000',
    JSON_OBJECT('shelf', 'B2', 'series', 'Earthsea')
  ),
  (
    102,
    2,
    '9780316229296',
    'The Fifth Season',
    'The first novel in the Broken Earth trilogy.',
    'paperback',
    18.00,
    8,
    3,
    '2015-08-04',
    '2026-01-03 10:00:00.000000',
    JSON_OBJECT('shelf', 'C3', 'series', 'The Broken Earth')
  ),
  (
    103,
    3,
    '9780446675505',
    'Parable of the Sower',
    'A near-future novel about survival and belief.',
    'ebook',
    11.99,
    0,
    5,
    '1993-10-01',
    '2026-01-04 10:00:00.000000',
    JSON_OBJECT('shelf', 'DIGITAL', 'series', 'Earthseed')
  ),
  (
    104,
    3,
    '9780446603775',
    'Kindred',
    NULL,
    'paperback',
    14.99,
    5,
    2,
    '1979-06-01',
    '2026-01-05 10:00:00.000000',
    NULL
  ),
  (
    105,
    4,
    '9780156453806',
    'Invisible Cities',
    'A sequence of imagined cities.',
    'paperback',
    15.95,
    1,
    3,
    '1972-01-01',
    '2026-01-06 10:00:00.000000',
    JSON_OBJECT('shelf', 'D4', 'series', NULL)
  );

INSERT INTO bookstore_book_categories (
  book_id,
  category_id
) VALUES
  (100, 10),
  (100, 13),
  (101, 11),
  (102, 11),
  (102, 13),
  (103, 10),
  (104, 10),
  (105, 12);

INSERT INTO bookstore_orders (
  id,
  customer_id,
  order_number,
  status,
  currency,
  placed_at,
  paid_at,
  shipped_at,
  shipping_method,
  gift_message
) VALUES
  (
    5000,
    1000,
    'BK-1000',
    'delivered',
    'USD',
    '2026-03-04 10:11:12.123456',
    '2026-03-04 10:12:00.000000',
    '2026-03-05 08:00:00',
    'ground',
    NULL
  ),
  (
    5001,
    1000,
    'BK-1001',
    'paid',
    'USD',
    '2026-04-02 14:30:00.000000',
    '2026-04-02 14:32:00.000000',
    NULL,
    'priority',
    'Happy birthday'
  ),
  (
    5002,
    1001,
    'BK-1002',
    'shipped',
    'USD',
    '2026-04-10 09:15:00.000000',
    '2026-04-10 09:16:00.000000',
    '2026-04-11 07:00:00',
    'ground',
    NULL
  ),
  (
    5003,
    1002,
    'BK-1003',
    'draft',
    'USD',
    '2026-04-12 11:00:00.000000',
    NULL,
    NULL,
    NULL,
    NULL
  );

INSERT INTO bookstore_order_items (
  id,
  order_id,
  book_id,
  quantity,
  unit_price,
  discount_amount
) VALUES
  (9000, 5000, 100, 1, 16.99, NULL),
  (9001, 5000, 102, 1, 18.00, 2.00),
  (9002, 5001, 101, 1, 22.50, NULL),
  (9003, 5001, 105, 2, 15.95, 3.00),
  (9004, 5002, 104, 1, 14.99, NULL),
  (9005, 5003, 103, 1, 11.99, NULL);

INSERT INTO bookstore_reviews (
  id,
  book_id,
  customer_id,
  order_item_id,
  rating,
  review_title,
  review_body,
  approved,
  created_at
) VALUES
  (7000, 100, 1000, 9000, 5, 'Still brilliant', 'A thoughtful classic.', 1, '2026-03-10 09:00:00.000000'),
  (7001, 102, 1000, 9001, 5, 'Compelling', NULL, 1, '2026-03-11 09:00:00.000000'),
  (7002, 104, 1001, 9004, 4, NULL, 'Powerful and direct.', 1, '2026-04-12 09:00:00.000000'),
  (7003, 101, 1000, 9002, 4, 'Pending moderation', NULL, 0, '2026-04-13 09:00:00.000000');
