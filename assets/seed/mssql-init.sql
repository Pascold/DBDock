-- DBDock: creates the test database, login and user on SQL Server.
-- Executed by the mssql-init container on every start. The scripting
-- variables come from that container's environment variables.
SET NOCOUNT ON;

IF DB_ID(N'$(DBDOCK_DB)') IS NULL
    CREATE DATABASE [$(DBDOCK_DB)];
GO

IF SUSER_ID(N'$(DBDOCK_USER)') IS NULL
    CREATE LOGIN [$(DBDOCK_USER)]
        WITH PASSWORD = N'$(DBDOCK_PASSWORD)',
             CHECK_POLICY = OFF,
             DEFAULT_DATABASE = [$(DBDOCK_DB)];
ELSE
    -- Keep the login in sync when the password is changed in DBDock.
    ALTER LOGIN [$(DBDOCK_USER)] WITH PASSWORD = N'$(DBDOCK_PASSWORD)';
GO

USE [$(DBDOCK_DB)];
GO

IF USER_ID(N'$(DBDOCK_USER)') IS NULL
    CREATE USER [$(DBDOCK_USER)] FOR LOGIN [$(DBDOCK_USER)];
GO

ALTER ROLE db_owner ADD MEMBER [$(DBDOCK_USER)];
GO
