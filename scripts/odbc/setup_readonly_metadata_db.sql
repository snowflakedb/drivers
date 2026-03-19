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
