-- DBDock sample data for SQLite.
-- Runs once, when the database file is created.
-- Accented letters and other non-ASCII characters test encoding.
PRAGMA foreign_keys = ON;

CREATE TABLE customers (
    id         INTEGER PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name  TEXT NOT NULL,
    email      TEXT NOT NULL UNIQUE,
    city       TEXT,
    notes      TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE orders (
    id          INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL REFERENCES customers (id),
    product     TEXT    NOT NULL,
    quantity    INTEGER NOT NULL CHECK (quantity > 0),
    price       NUMERIC NOT NULL,
    order_date  TEXT    NOT NULL
);

CREATE VIEW v_customer_summary AS
SELECT c.id AS customer_id,
       c.first_name,
       c.last_name,
       c.city,
       COUNT(o.id) AS order_count,
       COALESCE(SUM(o.quantity * o.price), 0) AS order_total
FROM customers c
LEFT JOIN orders o ON o.customer_id = c.id
GROUP BY c.id, c.first_name, c.last_name, c.city;

INSERT INTO customers (first_name, last_name, email, city, notes) VALUES
    ('Zoë', 'Müller', 'zoe@example.com', 'Zürich', 'UTF-8 test: Zażółć gęślą jaźń · Straße · Ærøskøbing · €'),
    ('Emma', 'Johnson', 'emma@example.com', 'London', NULL),
    ('José', 'García', 'jose@example.com', 'Madrid', 'Regular customer'),
    ('Łukasz', 'Nowak-Kowalski', 'lukasz@example.com', 'Kraków', NULL),
    ('Chloé', 'Dubois', 'chloe@example.com', 'Paris', 'Pays by bank transfer');

INSERT INTO orders (customer_id, product, quantity, price, order_date) VALUES
    (1, 'Paperback novel', 1, 39.99, '2024-01-15'),
    (1, 'Gel pen', 10, 2.50, '2024-02-03'),
    (2, 'Coffee beans 1 kg', 2, 64.90, '2024-02-11'),
    (3, 'Wireless headphones', 1, 249.00, '2024-03-01'),
    (4, 'Squared notebook', 5, 4.20, '2024-03-07'),
    (5, 'Bath towel', 3, 29.99, '2024-03-20'),
    (2, 'Green tea', 4, 12.75, '2024-04-02'),
    (3, 'Teaspoons', 6, 7.10, '2024-04-18');
