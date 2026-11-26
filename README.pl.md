<div align="center">

<img src="assets/icon/dbdock-512.png" alt="Logo DBDock" width="128">

# DBDock

**Testowe serwery baz danych w Dockerze, uruchamiane jednym kliknięciem.**

[English](README.md) | Polski

[![CI](https://github.com/Pascold/DBDock/actions/workflows/ci.yml/badge.svg)](https://github.com/Pascold/DBDock/actions/workflows/ci.yml)
[![Najnowsze wydanie](https://img.shields.io/github/v/release/Pascold/DBDock?sort=semver)](https://github.com/Pascold/DBDock/releases/latest)
[![Pobrania](https://img.shields.io/github/downloads/Pascold/DBDock/total)](https://github.com/Pascold/DBDock/releases)
[![Licencja: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#licencja)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-orange?logo=rust)](https://www.rust-lang.org)
[![Platformy](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#instalacja)
[![PR mile widziane](https://img.shields.io/badge/PRs-welcome-brightgreen)](CONTRIBUTING.md)

<img src="docs/screenshot.png" alt="Główne okno DBDock" width="820">

</div>

DBDock to mała, jednoplikowa aplikacja desktopowa dla programistów, którzy
testują swoje programy — klienty SQL, ORM-y, migracje, sterowniki — na wielu
silnikach baz danych. Generuje plik `docker-compose.yml` i steruje
`docker compose` za Ciebie, więc nie musisz znać Dockera. Wystarczy, że
masz go zainstalowanego.

## Funkcje

- **Siedem silników**: PostgreSQL, MariaDB, MySQL, MS SQL Server, Firebird,
  InterBase i SQLite, każdy w osobnym kontenerze.
- **Jedno kliknięcie**, żeby uruchomić, zatrzymać, zrestartować albo
  zresetować bazę, lub uruchomić od razu wszystkie włączone. Status na żywo
  (Gotowa, Uruchamianie, Zatrzymana, ...) z healthchecków kontenerów.
- **Dane połączenia gotowe do skopiowania**: host, port, użytkownik, hasło
  i connection stringi (URL, JDBC, ADO.NET, Firebird, sqlcmd).
- **Dane testowe** z polskimi znakami (ąćęłńóśźż), które wyłapują błędy
  kodowania: tabele `klienci` i `zamowienia` oraz widok `v_podsumowanie`.
- **Ustawienia każdej bazy**: wersja obrazu, port, nazwa bazy, użytkownik
  i hasła, z walidacją na żywo (konflikty portów, zajęte porty, polityka
  haseł SQL Server).
- **Logi** każdej bazy, pełne wyjście każdego polecenia Dockera i czytelne
  komunikaty błędów.
- **Bezpieczne ustawienia domyślne**: porty są wystawiane tylko na
  `127.0.0.1`, a wszystko działa w osobnym projekcie compose `dbdock`.
- **Eksport** wygenerowanego `docker-compose.yml`, żeby używać go bez DBDock.
- **Informacja o aktualizacji**: czerwony znacznik „Nieaktualna wersja” na
  dolnym pasku prowadzi do najnowszego wydania. DBDock wysyła w tym celu przy
  starcie jedno anonimowe zapytanie do publicznego API GitHuba (można to
  wyłączyć polem „Sprawdzaj aktualizacje” na dolnym pasku); nic więcej nie
  opuszcza Twojego komputera.
- Interfejs po polsku i angielsku, jasny i ciemny motyw, bez instalatora.

## Obsługiwane bazy

| Silnik | Domyślny obraz | Port hosta → kontenera | Baza | Użytkownik / hasło | Administrator |
|---|---|---|---|---|---|
| PostgreSQL | `postgres:17` | 5432 → 5432 | `testdb` | `tester` / `tester123` | — (użytkownik jest superuserem) |
| MariaDB | `mariadb:11.4` | 3306 → 3306 | `testdb` | `tester` / `tester123` | `root` / `root123` |
| MySQL | `mysql:8.4` | 3307 → 3306 | `testdb` | `tester` / `tester123` | `root` / `root123` |
| MS SQL Server | `mcr.microsoft.com/mssql/server:2022-latest` | 1433 → 1433 | `testdb` | `tester` / `tester123` | `sa` / `Test_Passw0rd!` |
| Firebird | `firebirdsql/firebird:5` | 3050 → 3050 | `/var/lib/firebird/data/testdb.fdb` | `tester` / `tester123` | `SYSDBA` / `masterkey` |
| InterBase | `radstudio/interbase:2020.4` | 3051 → 3050 | tworzysz sam (patrz niżej) | — | `SYSDBA` / `masterkey` |
| SQLite | `alpine:3` + `sqlite` | — | `test.db` w folderze danych | — | — |

Każdą wartość można zmienić w **Ustawieniach** bazy. PostgreSQL 18 i nowsze
trzymają dane w innym katalogu; DBDock dobiera ścieżkę wolumenu do wersji
automatycznie.

## Wymagania

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) na
  Windows i macOS albo [Docker Engine](https://docs.docker.com/engine/install/)
  z wtyczką Compose (`docker compose`) na Linuksie.
- Miejsce na dysku na obrazy (sam SQL Server to około 1,5 GB).

DBDock sprawdza Dockera przy starcie i wyjaśnia, czego brakuje.

## Instalacja

Pobierz plik dla swojego systemu z
[najnowszego wydania](https://github.com/Pascold/DBDock/releases/latest):

| System | Plik |
|---|---|
| Windows x86_64 | `dbdock-vX.Y.Z-windows-x86_64.exe` |
| Linux x86_64 | `dbdock-vX.Y.Z-linux-x86_64.tar.gz` |
| macOS Apple Silicon (eksperymentalnie) | `dbdock-vX.Y.Z-macos-aarch64.tar.gz` |
| macOS Intel (eksperymentalnie) | `dbdock-vX.Y.Z-macos-x86_64.tar.gz` |

Nie ma instalatora: umieść plik wykonywalny w dowolnym miejscu i uruchom go.
Sumy kontrolne są w `SHA256SUMS.txt`.

- **Windows**: plik nie jest jeszcze podpisany, więc SmartScreen może
  ostrzec przy pierwszym uruchomieniu (*Więcej informacji → Uruchom mimo to*).
- **macOS**: plik nie jest notaryzowany. Po rozpakowaniu jednorazowo usuń
  flagę kwarantanny: `xattr -d com.apple.quarantine ./dbdock`.
- **Linux**: `tar xzf dbdock-*.tar.gz && ./dbdock-*/dbdock`.

> **Obsługa macOS jest eksperymentalna.** Wersje na macOS kompilują się i
> przechodzą wszystkie testy automatyczne na maszynach macOS w GitHub Actions,
> ale nie były jeszcze uruchamiane na prawdziwym Macu. Jeśli masz Maca,
> wypróbuj DBDock i daj znać, jak poszło, w
> [Dyskusjach](https://github.com/Pascold/DBDock/discussions) albo w
> [zgłoszeniu](https://github.com/Pascold/DBDock/issues/new/choose) — każda
> informacja zwrotna jest mile widziana.

## Użycie

1. Uruchom Dockera, potem DBDock. Zielona kontrolka **Docker** oznacza
   gotowość.
2. Kliknij **Uruchom** na karcie (albo **Uruchom włączone**). Pierwszy start
   pobiera obraz; kliknij spinner, żeby zobaczyć postęp.
3. Gdy karta pokazuje **Gotowa**, otwórz **Połączenie** albo kliknij
   **Kopiuj**, żeby skopiować connection string.
4. **Zatrzymaj** zachowuje dane, **Resetuj dane** je usuwa (po
   potwierdzeniu), **Logi** pokazują wyjście bazy.
5. Pole wyboru na karcie włącza lub wyłącza bazę dla **Uruchom włączone**
   i dla eksportowanego pliku compose.

Zmiana ustawień istniejącej bazy: **Zastosuj i zrestartuj** tworzy kontener
od nowa. Nazwa bazy, użytkownik i hasła są używane tylko przy tworzeniu
danych, więc po ich zmianie wybierz **Zastosuj i zresetuj dane**.

### Gdzie są moje pliki?

| System | Folder danych |
|---|---|
| Windows | `%APPDATA%\DBDock\config` |
| macOS | `~/Library/Application Support/DBDock` |
| Linux | `~/.config/dbdock` |

Zawiera `config.toml`, `window.ron` (rozmiar i pozycja okna), wygenerowany
`docker-compose.yml`, skrypty seed i folder
`sqlite`. Otwiera go przycisk **Otwórz folder danych** na górnym pasku.
Zmienna środowiskowa `DBDOCK_HOME` pozwala użyć innego folderu.

> **Hasła są zapisywane jawnym tekstem** w `config.toml` i
> `docker-compose.yml`. DBDock służy do lokalnych baz testowych; nigdy nie
> używaj w nim danych logowania z produkcji.

### Plik compose bez DBDock

**Eksportuj compose** zapisuje `dbdock-docker-compose.yml` w folderze Pobrane.
Plik odwołuje się do skryptów seed w folderze danych, więc zostaw folder
danych DBDock na miejscu i uruchom:

```sh
docker compose -f dbdock-docker-compose.yml up -d
```

InterBase jest w osobnym profilu i startuje tylko z `--profile interbase`.

## Połączenie z aplikacji

Dla ustawień domyślnych:

| Silnik | Connection string |
|---|---|
| PostgreSQL | `postgres://tester:tester123@localhost:5432/testdb` |
| PostgreSQL (JDBC) | `jdbc:postgresql://localhost:5432/testdb` |
| MariaDB | `mariadb://tester:tester123@localhost:3306/testdb` |
| MySQL | `mysql://tester:tester123@localhost:3307/testdb` |
| MySQL (ADO.NET) | `Server=localhost;Port=3307;Database=testdb;Uid=tester;Pwd=tester123` |
| MS SQL Server (ADO.NET) | `Server=localhost,1433;Database=testdb;User Id=tester;Password=tester123;TrustServerCertificate=True` |
| MS SQL Server (JDBC) | `jdbc:sqlserver://localhost:1433;databaseName=testdb;encrypt=true;trustServerCertificate=true` |
| Firebird | `localhost/3050:/var/lib/firebird/data/testdb.fdb` |
| Firebird (JDBC) | `jdbc:firebird://localhost:3050//var/lib/firebird/data/testdb.fdb` |
| InterBase | `localhost/3051:/opt/interbase/testdb.ib` |
| SQLite | plik `sqlite/test.db` w folderze danych |

Okno **Połączenie** pokazuje te wartości dla bieżących ustawień, a znaki
specjalne w hasłach są tam, gdzie trzeba, zakodowane w URL.

## Licencja InterBase

InterBase to komercyjne oprogramowanie firmy Embarcadero. Oficjalny obraz
`radstudio/interbase` wymaga licencji; do testów wystarczy darmowa
**Developer Edition**. Dlatego InterBase jest domyślnie wyłączony, a jego
karta oferuje **Skonfiguruj licencję…** — krótki kreator, który:

1. prowadzi do [rejestracji Developer Edition](https://www.embarcadero.com/products/interbase/start-for-free)
   (numer seryjny przychodzi e-mailem);
2. pokazuje polecenie uruchamiające interaktywnie menedżer licencji (skopiuj
   je albo pozwól DBDock otworzyć terminal). Wybierz **[2] Direct register**,
   potem **[4] Save** i **[5] Quit**: bez Save nic się nie zapisze. Licencja
   trafia do wolumenu `dbdock_interbase-license` i przetrwa **Resetuj dane**;
3. uruchamia InterBase normalnie po potwierdzeniu rejestracji;
4. pokazuje, jak utworzyć bazę w `isql`, np.
   `CREATE DATABASE '/opt/interbase/testdb.ib';`.

Jeśli InterBase wystartuje bez zarejestrowanej licencji, kontener od razu się
zatrzyma, a karta pokaże **Brak licencji** i znów przycisk
**Skonfiguruj licencję…**. Gdy serwer przyjmuje połączenia, karta pokazuje
**Gotowa**.

Obraz istnieje tylko dla x86-64.

## FAQ / Rozwiązywanie problemów

**„Inny program nasłuchuje na tym porcie”** — port zajmuje coś innego
(często lokalnie zainstalowana baza). Zatrzymaj to albo wybierz inny port w
**Ustawieniach**; connection stringi zmienią się same.

**SQL Server na Apple Silicon (ARM)** — Microsoft publikuje tylko obrazy
x86-64. DBDock uruchamia je z `platform: linux/amd64`; włącz w ustawieniach
Docker Desktop *Use Rosetta for x86_64/amd64 emulation*. Start jest
wolniejszy, a niektóre wersje SQL Server mogą w ogóle nie działać w
emulacji. To samo dotyczy InterBase.

**„The certificate chain was issued by an authority that is not trusted”**
— SQL Server używa certyfikatu z podpisem własnym. Dodaj
`TrustServerCertificate=True` (ADO.NET), `trustServerCertificate=true`
(JDBC) albo `-C` (sqlcmd).

**SQL Server wisi na „Uruchamianie”** — serwer wymaga silnego hasła SA
(8+ znaków, trzy z kategorii: wielkie litery, małe litery, cyfry, symbole);
DBDock to sprawdza, ale zajrzyj do **Logów**, jeśli edytowałeś plik compose
ręcznie. Pierwszy start może potrwać minutę.

**Zmieniłem hasło, a działa stare** — oficjalne obrazy tworzą użytkowników
tylko wtedy, gdy wolumen z danymi jest pusty. Użyj **Zastosuj i zresetuj
dane** (albo **Resetuj dane**), żeby zacząć od zera.

**Jak zresetować bazę?** — **Resetuj dane** na karcie usuwa kontener i jego
wolumen; następny start inicjalizuje bazę od nowa razem z danymi testowymi.
Dla SQLite usuwany jest plik.

**Docker nie jest wykrywany** — upewnij się, że Docker Desktop działa (na
Linuksie usługa `docker`, a Twój użytkownik jest w grupie `docker`), i kliknij
**Sprawdź ponownie**. Na macOS DBDock szuka też w `/usr/local/bin`,
`/opt/homebrew/bin` i `~/.docker/bin`.

**Czy inne komputery mogą się połączyć?** — Nie, porty są celowo przypięte do
`127.0.0.1`.

## Budowanie ze źródeł

```sh
git clone https://github.com/Pascold/DBDock.git
cd DBDock
cargo build --release
```

Plik wynikowy to `target/release/dbdock` (`dbdock.exe` na Windows). Na
Linuksie najpierw zainstaluj `libxcb-render0-dev libxcb-shape0-dev
libxcb-xfixes0-dev libxkbcommon-dev`. Wymagany jest Rust 1.95 lub nowszy.

Testy: `cargo test`; z dostępnym Dockerem także `cargo test -- --ignored`,
który sprawdza wygenerowane pliki przez `docker compose config`.

## Plany

- Kolejne silniki: Oracle Database Free, IBM Db2, ClickHouse, MongoDB, Redis.
- Kilka instancji lub wersji tego samego silnika jednocześnie.
- Własne skrypty inicjalizacyjne dla każdej bazy.
- Opcjonalny dostęp z sieci lokalnej (`0.0.0.0`) dla wybranych baz.
- Podpisane i notaryzowane wydania, paczka aplikacji dla macOS.

Pomysły i pull requesty są mile widziane.

## Współtworzenie

Zobacz [CONTRIBUTING.md](CONTRIBUTING.md) i
[Kodeks postępowania](CODE_OF_CONDUCT.md). Problemy bezpieczeństwa:
[SECURITY.md](SECURITY.md).

## Licencja

Do wyboru jedna z licencji:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- licencja MIT ([LICENSE-MIT](LICENSE-MIT))

DBDock jedynie zarządza oficjalnymi obrazami baz danych; każda baza ma własną
licencję (SQL Server Developer Edition i InterBase Developer Edition są
darmowe wyłącznie do celów deweloperskich i testowych).
