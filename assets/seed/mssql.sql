-- DBDock sample data for SQL Server.
-- Executed by the mssql-init container on every start, so it skips itself
-- once the tables exist.
-- Accented letters and other non-ASCII characters test encoding.
SET NOCOUNT ON;
-- klienci: the Polish sample tables of DBDock 0.1.x, don't add a second set.
IF OBJECT_ID(N'dbo.customers', N'U') IS NOT NULL
   OR OBJECT_ID(N'dbo.klienci', N'U') IS NOT NULL
    SET NOEXEC ON;
GO

CREATE TABLE dbo.customers (
    id         INT IDENTITY(1,1) PRIMARY KEY,
    first_name NVARCHAR(50)  NOT NULL,
    last_name  NVARCHAR(80)  NOT NULL,
    email      NVARCHAR(120) NOT NULL UNIQUE,
    city       NVARCHAR(80)  NULL,
    notes      NVARCHAR(200) NULL,
    created_at DATETIME2     NOT NULL DEFAULT SYSDATETIME()
);

CREATE TABLE dbo.orders (
    id          INT IDENTITY(1,1) PRIMARY KEY,
    customer_id INT           NOT NULL REFERENCES dbo.customers (id),
    product     NVARCHAR(120) NOT NULL,
    quantity    INT           NOT NULL CHECK (quantity > 0),
    price       DECIMAL(10,2) NOT NULL,
    order_date  DATE          NOT NULL
);
GO

CREATE VIEW dbo.v_customer_summary AS
SELECT c.id AS customer_id,
       c.first_name,
       c.last_name,
       c.city,
       COUNT(o.id) AS order_count,
       COALESCE(SUM(o.quantity * o.price), 0) AS order_total
FROM dbo.customers c
LEFT JOIN dbo.orders o ON o.customer_id = c.id
GROUP BY c.id, c.first_name, c.last_name, c.city;
GO

INSERT INTO dbo.customers (first_name, last_name, email, city, notes) VALUES
    (N'Zoë', N'Müller', N'zoe@example.com', N'Zürich', N'UTF-8 test: Zażółć gęślą jaźń · Straße · Ærøskøbing · €'),
    (N'Emma', N'Johnson', N'emma@example.com', N'London', NULL),
    (N'José', N'García', N'jose@example.com', N'Madrid', N'Regular customer'),
    (N'Łukasz', N'Nowak-Kowalski', N'lukasz@example.com', N'Kraków', NULL),
    (N'Chloé', N'Dubois', N'chloe@example.com', N'Paris', N'Pays by bank transfer');

INSERT INTO dbo.orders (customer_id, product, quantity, price, order_date) VALUES
    (1, N'Paperback novel', 1, 39.99, '2024-01-15'),
    (1, N'Gel pen', 10, 2.50, '2024-02-03'),
    (2, N'Coffee beans 1 kg', 2, 64.90, '2024-02-11'),
    (3, N'Wireless headphones', 1, 249.00, '2024-03-01'),
    (4, N'Squared notebook', 5, 4.20, '2024-03-07'),
    (5, N'Bath towel', 3, 29.99, '2024-03-20'),
    (2, N'Green tea', 4, 12.75, '2024-04-02'),
    (3, N'Teaspoons', 6, 7.10, '2024-04-18');
GO

SET NOEXEC OFF;
GO
