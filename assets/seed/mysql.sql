-- DBDock sample data for MySQL.
-- Runs once, when the data volume is created. Polish characters test encoding.
SET NAMES utf8mb4;

CREATE TABLE klienci (
    id        INT          NOT NULL AUTO_INCREMENT PRIMARY KEY,
    imie      VARCHAR(50)  NOT NULL,
    nazwisko  VARCHAR(80)  NOT NULL,
    email     VARCHAR(120) NOT NULL UNIQUE,
    miasto    VARCHAR(80),
    uwagi     VARCHAR(200),
    utworzono TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE zamowienia (
    id              INT           NOT NULL AUTO_INCREMENT PRIMARY KEY,
    klient_id       INT           NOT NULL,
    produkt         VARCHAR(120)  NOT NULL,
    ilosc           INT           NOT NULL CHECK (ilosc > 0),
    cena            DECIMAL(10,2) NOT NULL,
    data_zamowienia DATE          NOT NULL,
    CONSTRAINT fk_zamowienia_klienci FOREIGN KEY (klient_id) REFERENCES klienci (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE VIEW v_podsumowanie AS
SELECT k.id AS klient_id,
       k.imie,
       k.nazwisko,
       k.miasto,
       COUNT(z.id) AS liczba_zamowien,
       COALESCE(SUM(z.ilosc * z.cena), 0) AS wartosc_zamowien
FROM klienci k
LEFT JOIN zamowienia z ON z.klient_id = k.id
GROUP BY k.id, k.imie, k.nazwisko, k.miasto;

INSERT INTO klienci (imie, nazwisko, email, miasto, uwagi) VALUES
    ('Łukasz', 'Żółkiewski', 'lukasz@example.com', 'Łódź', 'Zażółć gęślą jaźń / ZAŻÓŁĆ GĘŚLĄ JAŹŃ'),
    ('Ewa', 'Wiśniewska', 'ewa@example.com', 'Kraków', NULL),
    ('Michał', 'Dąbrowski', 'michal@example.com', 'Gdańsk', 'Stały klient'),
    ('Agnieszka', 'Zięba-Kołodziej', 'agnieszka@example.com', 'Poznań', NULL),
    ('Józef', 'Ślusarczyk', 'jozef@example.com', 'Częstochowa', 'Płatność przelewem');

INSERT INTO zamowienia (klient_id, produkt, ilosc, cena, data_zamowienia) VALUES
    (1, 'Książka „Pan Tadeusz”', 1, 39.99, '2024-01-15'),
    (1, 'Długopis żelowy', 10, 2.50, '2024-02-03'),
    (2, 'Kawa ziarnista 1 kg', 2, 64.90, '2024-02-11'),
    (3, 'Słuchawki bezprzewodowe', 1, 249.00, '2024-03-01'),
    (4, 'Zeszyt w kratkę', 5, 4.20, '2024-03-07'),
    (5, 'Ręcznik kąpielowy', 3, 29.99, '2024-03-20'),
    (2, 'Herbata źródlana', 4, 12.75, '2024-04-02'),
    (3, 'Łyżeczki do herbaty', 6, 7.10, '2024-04-18');
