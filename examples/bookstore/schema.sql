DROP TABLE IF EXISTS bookstore_reviews;
DROP TABLE IF EXISTS bookstore_order_items;
DROP TABLE IF EXISTS bookstore_orders;
DROP TABLE IF EXISTS bookstore_book_categories;
DROP TABLE IF EXISTS bookstore_books;
DROP TABLE IF EXISTS bookstore_categories;
DROP TABLE IF EXISTS bookstore_customers;
DROP TABLE IF EXISTS bookstore_authors;

CREATE TABLE bookstore_authors (
  id BIGINT NOT NULL PRIMARY KEY,
  display_name VARCHAR(255) NOT NULL,
  country_code CHAR(2) NULL,
  biography TEXT NULL
);

CREATE TABLE bookstore_customers (
  id BIGINT NOT NULL PRIMARY KEY,
  email VARCHAR(320) NOT NULL,
  full_name VARCHAR(255) NOT NULL,
  loyalty_tier ENUM('standard', 'silver', 'gold') NOT NULL,
  joined_on DATE NOT NULL,
  marketing_opt_in TINYINT(1) NOT NULL,
  last_seen_at DATETIME(6) NULL,
  UNIQUE KEY uq_bookstore_customers_email (email)
);

CREATE TABLE bookstore_categories (
  id BIGINT NOT NULL PRIMARY KEY,
  slug VARCHAR(64) NOT NULL,
  display_name VARCHAR(255) NOT NULL,
  UNIQUE KEY uq_bookstore_categories_slug (slug)
);

CREATE TABLE bookstore_books (
  id BIGINT NOT NULL PRIMARY KEY,
  author_id BIGINT NOT NULL,
  isbn VARCHAR(20) NOT NULL,
  title VARCHAR(255) NOT NULL,
  description TEXT NULL,
  format ENUM('hardcover', 'paperback', 'ebook') NOT NULL,
  price DECIMAL(10, 2) NOT NULL,
  stock_quantity INT NOT NULL,
  reorder_level INT NOT NULL,
  published_on DATE NULL,
  created_at DATETIME(6) NOT NULL,
  metadata JSON NULL,
  UNIQUE KEY uq_bookstore_books_isbn (isbn),
  KEY idx_bookstore_books_author (author_id),
  CONSTRAINT fk_bookstore_books_author
    FOREIGN KEY (author_id) REFERENCES bookstore_authors (id)
);

CREATE TABLE bookstore_book_categories (
  book_id BIGINT NOT NULL,
  category_id BIGINT NOT NULL,
  PRIMARY KEY (book_id, category_id),
  CONSTRAINT fk_bookstore_book_categories_book
    FOREIGN KEY (book_id) REFERENCES bookstore_books (id),
  CONSTRAINT fk_bookstore_book_categories_category
    FOREIGN KEY (category_id) REFERENCES bookstore_categories (id)
);

CREATE TABLE bookstore_orders (
  id BIGINT NOT NULL PRIMARY KEY,
  customer_id BIGINT NOT NULL,
  order_number VARCHAR(32) NOT NULL,
  status ENUM('draft', 'paid', 'shipped', 'delivered', 'cancelled') NOT NULL,
  currency CHAR(3) NOT NULL,
  placed_at DATETIME(6) NOT NULL,
  paid_at DATETIME(6) NULL,
  shipped_at TIMESTAMP NULL DEFAULT NULL,
  shipping_method VARCHAR(64) NULL,
  gift_message TEXT NULL,
  UNIQUE KEY uq_bookstore_orders_number (order_number),
  KEY idx_bookstore_orders_customer (customer_id, placed_at),
  CONSTRAINT fk_bookstore_orders_customer
    FOREIGN KEY (customer_id) REFERENCES bookstore_customers (id)
);

CREATE TABLE bookstore_order_items (
  id BIGINT NOT NULL PRIMARY KEY,
  order_id BIGINT NOT NULL,
  book_id BIGINT NOT NULL,
  quantity INT NOT NULL,
  unit_price DECIMAL(10, 2) NOT NULL,
  discount_amount DECIMAL(10, 2) NULL,
  CONSTRAINT fk_bookstore_order_items_order
    FOREIGN KEY (order_id) REFERENCES bookstore_orders (id),
  CONSTRAINT fk_bookstore_order_items_book
    FOREIGN KEY (book_id) REFERENCES bookstore_books (id)
);

CREATE TABLE bookstore_reviews (
  id BIGINT NOT NULL PRIMARY KEY,
  book_id BIGINT NOT NULL,
  customer_id BIGINT NOT NULL,
  order_item_id BIGINT NULL,
  rating TINYINT NOT NULL,
  review_title VARCHAR(255) NULL,
  review_body TEXT NULL,
  approved TINYINT(1) NOT NULL,
  created_at DATETIME(6) NOT NULL,
  KEY idx_bookstore_reviews_book (book_id, approved),
  CONSTRAINT fk_bookstore_reviews_book
    FOREIGN KEY (book_id) REFERENCES bookstore_books (id),
  CONSTRAINT fk_bookstore_reviews_customer
    FOREIGN KEY (customer_id) REFERENCES bookstore_customers (id),
  CONSTRAINT fk_bookstore_reviews_order_item
    FOREIGN KEY (order_item_id) REFERENCES bookstore_order_items (id)
);
