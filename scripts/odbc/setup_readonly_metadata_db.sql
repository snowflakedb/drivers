-- =============================================================================
-- Readonly Metadata Test Database Setup
-- =============================================================================
--
-- This script creates a dedicated Snowflake database with pre-provisioned
-- tables, views, and procedures used by ODBC catalog and metadata tests.
--
-- Purpose: Eliminate per-test DDL that causes flaky test failures. Tests query
-- this pre-existing schema instead of creating their own objects.
--
-- IMPORTANT: Object names MUST NOT contain underscores. ODBC catalog functions
-- treat '_' as a single-character wildcard in pattern arguments, which causes
-- painfully slow metadata queries when identifiers contain them.
--
-- Usage:
--   Via the C++ runner (recommended):
--     cd odbc_tests
--     cmake -B cmake-build -DBUILD_SETUP_TOOLS=ON [other flags...]
--     cmake --build cmake-build
--     ctest --test-dir cmake-build -R setup_readonly_db
--
--   Via SnowSQL:
--     snowsql -f scripts/odbc/setup_readonly_metadata_db.sql
--
-- This script is idempotent: it drops and recreates the schema (CASCADE),
-- so it can be safely re-run to reset the database.
-- =============================================================================

CREATE DATABASE IF NOT EXISTS ODBCMETADATATESTDB;
USE DATABASE ODBCMETADATATESTDB;
DROP SCHEMA IF EXISTS CATALOGTESTS CASCADE;
CREATE SCHEMA CATALOGTESTS;
USE SCHEMA CATALOGTESTS;

-- =============================================================================
-- Basic tables (used by SQLTables, SQLColumns, SQLTablePrivileges,
-- SQLColumnPrivileges, and SQLDescribeCol tests)
-- =============================================================================

CREATE TABLE BASICTABLE (id INT, name VARCHAR(100));
CREATE TABLE MULTITYPETABLE (id INTEGER, name VARCHAR(100), price FLOAT, active BOOLEAN);
CREATE TABLE THREECOLTABLE (cola INT, colb VARCHAR(50), colc FLOAT);
CREATE TABLE NULLABILITYTABLE (id INTEGER NOT NULL, name VARCHAR(100));
CREATE TABLE WILDCARDCOLTABLE (ca INT, cb INT, ddd INT);
CREATE TABLE NOPKTABLE (id INT, name VARCHAR(50));

-- =============================================================================
-- Primary key tables (used by SQLPrimaryKeys, SQLStatistics,
-- SQLSpecialColumns tests)
-- =============================================================================

CREATE TABLE SINGLEPKTABLE (id INT PRIMARY KEY, name VARCHAR(50));
CREATE TABLE COMPOSITEPKTABLE (regionid INT, storeid INT, name VARCHAR(50), PRIMARY KEY (regionid, storeid));
CREATE TABLE NAMEDPKTABLE (id INT, CONSTRAINT PKNAMED PRIMARY KEY (id));

-- =============================================================================
-- Foreign key tables (used by SQLForeignKeys tests)
-- Parent tables must be created before children.
-- =============================================================================

CREATE TABLE FKPARENT (id INT PRIMARY KEY);
CREATE TABLE FKCHILD (id INT, parentid INT, FOREIGN KEY (parentid) REFERENCES FKPARENT(id));
CREATE TABLE FKMULTIPARENT (id INT PRIMARY KEY);
CREATE TABLE FKMULTICHILDA (id INT, parentid INT, FOREIGN KEY (parentid) REFERENCES FKMULTIPARENT(id));
CREATE TABLE FKMULTICHILDB (id INT, refid INT, FOREIGN KEY (refid) REFERENCES FKMULTIPARENT(id));

-- =============================================================================
-- Views (used by SQLTables VIEW type tests)
-- =============================================================================

CREATE VIEW BASICVIEW AS SELECT * FROM BASICTABLE;

-- =============================================================================
-- Procedures (used by SQLProcedures and SQLProcedureColumns tests)
-- =============================================================================

CREATE PROCEDURE BASICPROC(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END';
CREATE PROCEDURE MULTIPARAMPROC(pname VARCHAR, page FLOAT) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN pname; END';
CREATE PROCEDURE PROCFILTER(pid INTEGER, pname VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN pname; END';
CREATE PROCEDURE PROCMULTIA(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END';
CREATE PROCEDURE PROCMULTIB(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END';
CREATE PROCEDURE PROCDTYPEA(p1 VARCHAR) RETURNS VARCHAR LANGUAGE SQL AS 'BEGIN RETURN p1; END';
CREATE PROCEDURE PROCDTYPEB(p1 INT) RETURNS INT LANGUAGE SQL AS 'BEGIN RETURN p1; END';
CREATE PROCEDURE PROCNUMA(p1 INT) RETURNS INT LANGUAGE SQL AS 'BEGIN RETURN p1; END';
CREATE PROCEDURE PROCNUMB(p1 INT) RETURNS INT LANGUAGE SQL AS 'BEGIN RETURN p1; END';

-- =============================================================================
-- SQLDescribeCol tables (used by e2e/query/sql_describe_col.cpp)
-- =============================================================================

CREATE TABLE DESCVARCHARTABLE (val VARCHAR(100));
CREATE TABLE DESCNUMBERTABLE (val NUMBER(10,2));
CREATE TABLE DESCBOOLTABLE (val BOOLEAN);
CREATE TABLE DESCFLOATTABLE (val FLOAT);
CREATE TABLE DESCDATETABLE (val DATE);
CREATE TABLE DESCTIMESTAMPTABLE (val TIMESTAMP_NTZ);
CREATE TABLE DESCSIZEVARCHARTABLE (val VARCHAR(200));
CREATE TABLE DESCSIZENUMBERTABLE (val NUMBER(12,3));
CREATE TABLE DESCDIGITSTABLE (val NUMBER(10,4));
CREATE TABLE DESCDIGITSVARCHARTABLE (val VARCHAR(50));
CREATE TABLE DESCNULLABLETABLE (val VARCHAR(50));
CREATE TABLE DESCNOTNULLTABLE (val VARCHAR(50) NOT NULL);
CREATE TABLE DESCMULTITABLE (strcol VARCHAR(50), numcol NUMBER(8,2), boolcol BOOLEAN);

-- =============================================================================
-- DATATYPETESTS schema -- data-bearing fixtures for Excel/PQ trace replay tests.
-- Distinct from CATALOGTESTS because these tables carry rows (not just DDL)
-- and are exercised for type conversion, W-API encoding, and LOB streaming.
-- =============================================================================

DROP SCHEMA IF EXISTS DATATYPETESTS CASCADE;
CREATE SCHEMA DATATYPETESTS;
USE SCHEMA DATATYPETESTS;

CREATE TABLE ALLDATATYPES (
  ROWKIND       VARCHAR(16),     -- 'NORMAL' | 'BOUNDARY' | 'UNICODE' | 'NULLROW'
  INTVAL        INTEGER,
  BIGINTVAL     BIGINT,
  SMALLINTVAL   SMALLINT,
  TINYINTVAL    TINYINT,
  NUM38         NUMBER(38, 0),
  NUM18S6       NUMBER(18, 6),
  FLOATVAL      FLOAT,
  DOUBLEVAL     DOUBLE,
  REALVAL       REAL,
  VARCHARVAL    VARCHAR(256),
  TEXTVAL       TEXT,
  CHARVAL       CHAR(10),
  BINARYVAL     BINARY(16),
  VARBINARYVAL  VARBINARY,
  BOOLVAL       BOOLEAN,
  DATEVAL       DATE,
  TIMEVAL       TIME,
  TSNTZ         TIMESTAMP_NTZ,
  TSLTZ         TIMESTAMP_LTZ,
  TSTZ          TIMESTAMP_TZ,
  VARIANTVAL    VARIANT,
  OBJECTVAL     OBJECT,
  ARRAYVAL      ARRAY,
  GEOVAL        GEOGRAPHY
);

CREATE TABLE LARGELOBS (
  ROWKIND       VARCHAR(16),     -- 'FULL' | 'NULLROW'
  LARGEVARCHAR  VARCHAR,         -- ~128 KB payload in the FULL row
  LARGEBINARY   VARBINARY        -- ~256 KB payload in the FULL row
);

-- -----------------------------------------------------------------------------
-- ALLDATATYPES rows. One row per ROWKIND so trace-replay tests can target a
-- specific shape (representative / boundary / unicode / all-null) without
-- relying on row order.
-- -----------------------------------------------------------------------------

INSERT INTO ALLDATATYPES
SELECT
  'NORMAL',
  42,
  100000000000,
  1000,
  100,
  123456789012345678901234567890,
  12345.678901,
  3.14,
  2.718281828459045,
  1.4142135,
  'hello world',
  'representative text payload',
  'fixedchar',
  TO_BINARY('DEADBEEFDEADBEEFDEADBEEFDEADBEEF', 'HEX'),
  TO_BINARY('CAFEBABE', 'HEX'),
  TRUE,
  '2024-01-15'::DATE,
  '13:45:30'::TIME,
  '2024-01-15 13:45:30'::TIMESTAMP_NTZ,
  '2024-01-15 13:45:30'::TIMESTAMP_LTZ,
  '2024-01-15 13:45:30 -08:00'::TIMESTAMP_TZ,
  PARSE_JSON('{"a":1}'),
  OBJECT_CONSTRUCT('k', 'v'),
  ARRAY_CONSTRUCT(1, 2, 3),
  ST_GEOGRAPHYFROMTEXT('POINT(-122 37)');

INSERT INTO ALLDATATYPES
SELECT
  'BOUNDARY',
  2147483647,
  9223372036854775807,
  32767,
  127,
  99999999999999999999999999999999999999,
  999999999999.999999,
  'Infinity'::FLOAT,
  'NaN'::DOUBLE,
  '-Infinity'::REAL,
  RPAD('X', 256, 'X'),
  'boundary text payload',
  RPAD('Y', 10, 'Y'),
  TO_BINARY('FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF', 'HEX'),
  TO_BINARY('FFFF', 'HEX'),
  FALSE,
  '9999-12-31'::DATE,
  '23:59:59.999'::TIME,
  '9999-12-31 23:59:59.999999999'::TIMESTAMP_NTZ,
  '9999-12-31 23:59:59.999999999'::TIMESTAMP_LTZ,
  '9999-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_TZ,
  PARSE_JSON('null'),
  OBJECT_CONSTRUCT(),
  ARRAY_CONSTRUCT(),
  ST_GEOGRAPHYFROMTEXT('POINT(180 90)');

INSERT INTO ALLDATATYPES
SELECT
  'UNICODE',
  7,
  8,
  9,
  1,
  42,
  3.141593,
  1.0,
  2.0,
  3.0,
  'CJK 日本語 emoji 😀 RTL עברית quote"X backslash\X',
  '中文 العربية 한국어 🚀 mixed scripts',
  '汉字abc',
  TO_BINARY('00112233445566778899AABBCCDDEEFF', 'HEX'),
  TO_BINARY('CAFE', 'HEX'),
  TRUE,
  '2024-02-29'::DATE,
  '12:00:00'::TIME,
  '2024-02-29 12:00:00'::TIMESTAMP_NTZ,
  '2024-02-29 12:00:00'::TIMESTAMP_LTZ,
  '2024-02-29 12:00:00 +09:00'::TIMESTAMP_TZ,
  PARSE_JSON('{"emoji":"😀","cjk":"日本"}'),
  OBJECT_CONSTRUCT('lang', '中文'),
  ARRAY_CONSTRUCT('日', '本', '語'),
  ST_GEOGRAPHYFROMTEXT('POINT(139.6917 35.6895)');

INSERT INTO ALLDATATYPES
SELECT
  'NULLROW',
  NULL, NULL, NULL, NULL,
  NULL, NULL,
  NULL, NULL, NULL,
  NULL, NULL, NULL,
  NULL, NULL,
  NULL,
  NULL, NULL,
  NULL, NULL, NULL,
  NULL, NULL, NULL,
  NULL;

-- -----------------------------------------------------------------------------
-- LARGELOBS rows. FULL exercises chunked LOB streaming through SQLGetData;
-- NULLROW lets tests assert SQL_NULL_DATA on both LOB columns.
-- -----------------------------------------------------------------------------

INSERT INTO LARGELOBS
SELECT
  'FULL',
  RPAD('X', 131072, 'X'),
  TO_BINARY(REPEAT('DEADBEEF', 65536), 'HEX');

INSERT INTO LARGELOBS
SELECT
  'NULLROW',
  NULL,
  NULL;
