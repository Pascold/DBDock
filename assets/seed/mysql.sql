-- DBDock sample data for MySQL.
-- Runs once, when the data volume is created.
-- Accented letters and other non-ASCII characters test encoding.
SET NAMES utf8mb4;

CREATE TABLE customers (
    id         INT          NOT NULL AUTO_INCREMENT PRIMARY KEY,
    first_name VARCHAR(50)  NOT NULL,
    last_name  VARCHAR(80)  NOT NULL,
    email      VARCHAR(120) NOT NULL UNIQUE,
    city       VARCHAR(80),
    notes      VARCHAR(200),
    created_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE orders (
    id          INT           NOT NULL AUTO_INCREMENT PRIMARY KEY,
    customer_id INT           NOT NULL,
    product     VARCHAR(120)  NOT NULL,
    quantity    INT           NOT NULL CHECK (quantity > 0),
    price       DECIMAL(10,2) NOT NULL,
    order_date  DATE          NOT NULL,
    CONSTRAINT fk_orders_customers FOREIGN KEY (customer_id) REFERENCES customers (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

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
