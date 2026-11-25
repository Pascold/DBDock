-- DBDock sample data for SQL Server.
-- Executed by the mssql-init container on every start, so it skips itself
-- once the tables exist. Polish characters test encoding.
SET NOCOUNT ON;
IF OBJECT_ID(N'dbo.klienci', N'U') IS NOT NULL SET NOEXEC ON;
GO

CREATE TABLE dbo.klienci (
    id        INT IDENTITY(1,1) PRIMARY KEY,
    imie      NVARCHAR(50)  NOT NULL,
    nazwisko  NVARCHAR(80)  NOT NULL,
    email     NVARCHAR(120) NOT NULL UNIQUE,
    miasto    NVARCHAR(80)  NULL,
    uwagi     NVARCHAR(200) NULL,
    utworzono DATETIME2     NOT NULL DEFAULT SYSDATETIME()
);

CREATE TABLE dbo.zamowienia (
    id              INT IDENTITY(1,1) PRIMARY KEY,
    klient_id       INT           NOT NULL REFERENCES dbo.klienci (id),
    produkt         NVARCHAR(120) NOT NULL,
    ilosc           INT           NOT NULL CHECK (ilosc > 0),
    cena            DECIMAL(10,2) NOT NULL,
    data_zamowienia DATE          NOT NULL
);
GO

CREATE VIEW dbo.v_podsumowanie AS
SELECT k.id AS klient_id,
       k.imie,
       k.nazwisko,
       k.miasto,
       COUNT(z.id) AS liczba_zamowien,
       COALESCE(SUM(z.ilosc * z.cena), 0) AS wartosc_zamowien
FROM dbo.klienci k
LEFT JOIN dbo.zamowienia z ON z.klient_id = k.id
GROUP BY k.id, k.imie, k.nazwisko, k.miasto;
GO

INSERT INTO dbo.klienci (imie, nazwisko, email, miasto, uwagi) VALUES
    (N'Łukasz', N'Żółkiewski', N'lukasz@example.com', N'Łódź', N'Zażółć gęślą jaźń / ZAŻÓŁĆ GĘŚLĄ JAŹŃ'),
    (N'Ewa', N'Wiśniewska', N'ewa@example.com', N'Kraków', NULL),
    (N'Michał', N'Dąbrowski', N'michal@example.com', N'Gdańsk', N'Stały klient'),
    (N'Agnieszka', N'Zięba-Kołodziej', N'agnieszka@example.com', N'Poznań', NULL),
    (N'Józef', N'Ślusarczyk', N'jozef@example.com', N'Częstochowa', N'Płatność przelewem');

INSERT INTO dbo.zamowienia (klient_id, produkt, ilosc, cena, data_zamowienia) VALUES
    (1, N'Książka „Pan Tadeusz”', 1, 39.99, '2024-01-15'),
    (1, N'Długopis żelowy', 10, 2.50, '2024-02-03'),
    (2, N'Kawa ziarnista 1 kg', 2, 64.90, '2024-02-11'),
    (3, N'Słuchawki bezprzewodowe', 1, 249.00, '2024-03-01'),
    (4, N'Zeszyt w kratkę', 5, 4.20, '2024-03-07'),
    (5, N'Ręcznik kąpielowy', 3, 29.99, '2024-03-20'),
    (2, N'Herbata źródlana', 4, 12.75, '2024-04-02'),
    (3, N'Łyżeczki do herbaty', 6, 7.10, '2024-04-18');
GO

SET NOEXEC OFF;
GO
