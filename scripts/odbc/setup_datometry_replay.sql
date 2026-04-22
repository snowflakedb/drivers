-- =============================================================================
-- Datometry Replay Test Database Setup
-- =============================================================================
--
-- Creates a Snowflake database with tables matching the schemas referenced by
-- ODBC trace replay tests in odbc_tests/tests/replay/datometry/.
--
-- The replay traces used DESC TABLE, SHOW TABLES, SHOW OBJECTS, and metadata
-- queries captured from a Datometry workload. This script provisions the
-- required objects so those queries return results instead of 42S02 errors.
--
-- Usage:
--   Via the C++ runner (recommended):
--     cd odbc_tests
--     cmake -B cmake-build -DBUILD_SETUP_TOOLS=ON [other flags...]
--     cmake --build cmake-build
--     ctest --test-dir cmake-build -R setup_datometry_replay
--
--   Via SnowSQL:
--     snowsql -f scripts/odbc/setup_datometry_replay.sql
--
-- This script is idempotent: it drops and recreates the PUBLIC schema
-- (CASCADE), so it can be safely re-run to reset the database.
-- =============================================================================

CREATE DATABASE IF NOT EXISTS DTMREPLAYTESTDB;
USE DATABASE DTMREPLAYTESTDB;
DROP SCHEMA IF EXISTS PUBLIC CASCADE;
CREATE SCHEMA PUBLIC;
USE SCHEMA PUBLIC;

-- =============================================================================
-- Small tables (2-5 columns)
-- =============================================================================

CREATE TABLE "cv_test1" ("i" NUMBER(38,0), "j" NUMBER(38,0), "k" NUMBER(38,0));
CREATE TABLE "hq18673_t" ("id" NUMBER(38,0), "v" NUMBER(38,0));

-- =============================================================================
-- Medium tables (5-15 columns)
-- =============================================================================

CREATE TABLE "hq17907_accum" (
  "ii" NUMBER(7,4), "jj" NUMBER(5,3),
  "t1" VARCHAR(20), "t2" VARCHAR(20),
  "depth" NUMBER(38,0)
);

CREATE TABLE "dimcalendarfiscalweek" (
  "FiscalYear" NUMBER(38,0), "FiscalMonth" NUMBER(38,0),
  "FiscalWeek" NUMBER(38,0), "FiscalDay" DATE,
  "WeekStartDate" DATE, "WeekEndDate" DATE,
  "LastUpdated" TIMESTAMP_NTZ(6)
);

CREATE TABLE "v_star1" (
  "country" VARCHAR(20), "state" VARCHAR(10), "yr" NUMBER(38,0),
  "'Q1'_ss1" NUMBER(38,0), "'Q1'_sc" NUMBER(38,0),
  "'Q2'_ss1" NUMBER(38,0), "'Q2'_sc" NUMBER(38,0),
  "'Q3'_ss1" NUMBER(38,0), "'Q3'_sc" NUMBER(38,0)
);

CREATE TABLE "TBK_PARTY_ROLE_H_HQ_18538" (
  "Party_Role_KEY" NUMBER(38,0) NOT NULL, "Namespace_TXT" VARCHAR(40) NOT NULL,
  "Legacy1_ID" VARCHAR(255) NOT NULL, "Legacy2_ID" VARCHAR(255) NOT NULL,
  "Master_Namespace_TXT" VARCHAR(40),
  "Master_Legacy1_ID" VARCHAR(255), "Master_Legacy2_ID" VARCHAR(255),
  "Rec_Start" DATE NOT NULL, "Rec_End" DATE NOT NULL,
  "Rec_Stat" VARCHAR(1) NOT NULL, "Load_KEY" NUMBER(38,0) NOT NULL
);

CREATE TABLE "uma_table_4" (
  "col_1" VARCHAR(8) NOT NULL, "col_27" VARCHAR(5) NOT NULL, "col_32" VARCHAR(4) NOT NULL,
  "col_18" VARCHAR(11) NOT NULL, "col_26" VARCHAR(11) NOT NULL, "col_9" NUMBER(38,0) NOT NULL,
  "audit_col_2" VARCHAR(17) NOT NULL, "audit_col_1" DATE NOT NULL,
  "audit_col_4" TIME(0) NOT NULL, "audit_col_6" VARCHAR(17) NOT NULL,
  "audit_col_5" DATE NOT NULL, "audit_col_8" TIME(0) NOT NULL,
  "audit_col_3" NUMBER(38,0), "audit_col_7" NUMBER(38,0)
);

-- =============================================================================
-- Large tables (18+ columns)
-- =============================================================================

CREATE TABLE "HQ_19541_tbl" (
  "id" NUMBER(38,0),
  "interval_col" NUMBER(38,0), "interval_col_year" NUMBER(38,0),
  "interval_col1" NUMBER(38,0), "interval_col2" NUMBER(38,0),
  "interval_col_month" NUMBER(38,0), "interval_col3" NUMBER(38,0),
  "interval_col_day" NUMBER(38,0), "interval_col4" NUMBER(38,0),
  "interval_col5" NUMBER(38,0), "interval_col6" NUMBER(38,0),
  "interval_col8" NUMBER(38,0), "interval_col_hour" NUMBER(38,0),
  "interval_col9" NUMBER(38,0), "interval_col10" NUMBER(38,0),
  "interval_col11" NUMBER(38,0), "interval_col_minute" NUMBER(38,0),
  "interval_col12" NUMBER(38,0)
);

CREATE TABLE "iceberg_types_tbl" (
  "col_integer" NUMBER(38,0), "col_smallint" NUMBER(38,0),
  "col_bigint" NUMBER(38,0), "col_byteint" NUMBER(38,0),
  "col_number" FLOAT, "col_number_ps" NUMBER(10,2),
  "col_decimal" NUMBER(10,2),
  "col_varchar" VARCHAR(134217728), "col_char" VARCHAR(134217728),
  "col_unicode_varchar" VARCHAR(134217728), "col_unicode_char" VARCHAR(134217728),
  "col_clob" VARCHAR(134217728),
  "col_varbyte" BINARY(67108864), "col_blob" BINARY(67108864),
  "col_date" DATE,
  "col_timestamp" TIMESTAMP_NTZ(6), "col_timestamp_6" TIMESTAMP_NTZ(6),
  "col_timestamp_0" TIMESTAMP_NTZ(6),
  "col_timestamp_tz" TIMESTAMP_LTZ(6), "col_timestamp_tz_6" TIMESTAMP_LTZ(6),
  "col_timestamp_tz_3" TIMESTAMP_LTZ(6),
  "col_time" TIME(6), "col_time_6" TIME(6), "col_time_3" TIME(6),
  "col_new_ts" TIMESTAMP_NTZ(6), "col_new_vc" VARCHAR(134217728)
);

CREATE TABLE "fcd_table_3" (
  "col_161" VARCHAR(100) NOT NULL, "col_173" VARCHAR(100) NOT NULL, "col_97" VARCHAR(20) NOT NULL,
  "col_164" VARCHAR(100) NOT NULL, "col_168" VARCHAR(100), "col_158" VARCHAR(25),
  "col_124" VARCHAR(25), "col_122" TIMESTAMP_NTZ(6),
  "col_121" VARCHAR(5) NOT NULL, "col_123" VARCHAR(5) NOT NULL, "col_102" VARCHAR(5) NOT NULL,
  "col_119" VARCHAR(1) NOT NULL, "col_140" NUMBER(38,0), "col_142" VARCHAR(25),
  "col_112" NUMBER(38,0), "col_34" VARCHAR(50),
  "col_141" TIMESTAMP_NTZ(6), "col_139" TIMESTAMP_NTZ(6),
  "col_52" VARCHAR(6) NOT NULL, "col_85" VARCHAR(6) NOT NULL, "col_133" VARCHAR(25),
  "col_82" TIMESTAMP_NTZ(6) NOT NULL, "col_54" DATE NOT NULL, "col_134" DATE NOT NULL,
  "audit_col_2" VARCHAR(17) NOT NULL, "audit_col_1" DATE NOT NULL,
  "audit_col_4" TIME(0) NOT NULL, "audit_col_6" VARCHAR(17) NOT NULL,
  "audit_col_5" DATE NOT NULL, "audit_col_8" TIME(0) NOT NULL,
  "audit_col_3" NUMBER(38,0), "audit_col_7" NUMBER(38,0)
);

CREATE TABLE "td_kw" (
  "committed" NUMBER(38,0), "access" NUMBER(38,0),
  "caller" NUMBER(38,0), "colocate" NUMBER(38,0),
  "data" NUMBER(38,0), "excl" NUMBER(38,0),
  "exclusive" NUMBER(38,0), "floor" NUMBER(38,0),
  "global" NUMBER(38,0), "high" NUMBER(38,0),
  "medium" NUMBER(38,0), "low" NUMBER(38,0),
  "last" NUMBER(38,0), "latin" NUMBER(38,0),
  "matched" NUMBER(38,0), "query_band" NUMBER(38,0),
  "range" NUMBER(38,0), "read" NUMBER(38,0),
  "stat" NUMBER(38,0), "stats" NUMBER(38,0),
  "system" NUMBER(38,0), "partition" NUMBER(38,0),
  "preceding" NUMBER(38,0), "unbounded" NUMBER(38,0),
  "unknown" NUMBER(38,0), "maxvalue" NUMBER(38,0),
  "minvalue" NUMBER(38,0), "share" NUMBER(38,0),
  "message_text" NUMBER(38,0), "to_number" NUMBER(38,0),
  "to_date" NUMBER(38,0), "to_char" NUMBER(38,0),
  "following" NUMBER(38,0), "workload" NUMBER(38,0),
  "year_month" NUMBER(38,0), "level" NUMBER(38,0),
  "creator" NUMBER(38,0), "owner" NUMBER(38,0),
  "invoker" NUMBER(38,0), "definer" NUMBER(38,0)
);

CREATE TABLE "prr_table_1" (
  "col_9" NUMBER(38,0) NOT NULL, "col_11" NUMBER(38,0), "col_13" NUMBER(38,0),
  "col_15" NUMBER(38,0), "col_17" NUMBER(38,0), "col_43" DATE NOT NULL,
  "col_32" VARCHAR(4) NOT NULL, "col_18" VARCHAR(11) NOT NULL, "col_1" VARCHAR(8) NOT NULL,
  "col_20" NUMBER(38,0) NOT NULL, "col_7" VARCHAR(3) NOT NULL, "col_38" VARCHAR(9) NOT NULL,
  "col_41" VARCHAR(10) NOT NULL, "col_22" VARCHAR(2) NOT NULL, "col_27" VARCHAR(5) NOT NULL,
  "col_21" VARCHAR(2) NOT NULL, "col_45" NUMBER(30,6), "col_33" NUMBER(30,6),
  "col_48" NUMBER(30,6), "col_26" VARCHAR(11) NOT NULL, "col_23" VARCHAR(6) NOT NULL,
  "col_34" VARCHAR(6) NOT NULL, "col_35" VARCHAR(15) NOT NULL, "col_37" VARCHAR(9) NOT NULL,
  "col_8" VARCHAR(31) NOT NULL, "col_3" VARCHAR(6) NOT NULL, "col_30" VARCHAR(6) NOT NULL,
  "col_19" VARCHAR(14) NOT NULL, "col_44" NUMBER(38,0), "col_29" NUMBER(38,0),
  "col_5" VARCHAR(25) NOT NULL, "col_40" VARCHAR(1) NOT NULL, "col_2" DATE,
  "col_28" VARCHAR(5) NOT NULL, "col_36" NUMBER(38,0) NOT NULL, "col_31" VARCHAR(9) NOT NULL,
  "col_25" VARCHAR(3) NOT NULL, "col_4" VARCHAR(50) NOT NULL, "col_47" VARCHAR(15) NOT NULL,
  "col_6" VARCHAR(5) NOT NULL, "col_39" NUMBER(38,0), "col_46" VARCHAR(9) NOT NULL,
  "audit_col_2" VARCHAR(17) NOT NULL, "audit_col_1" DATE NOT NULL,
  "audit_col_4" TIME(0) NOT NULL, "audit_col_6" VARCHAR(17) NOT NULL,
  "audit_col_5" DATE NOT NULL, "audit_col_8" TIME(0) NOT NULL,
  "audit_col_3" NUMBER(38,0), "audit_col_7" NUMBER(38,0)
);

CREATE TABLE "bpamain" (
  "Status" VARCHAR(1), "Dfu" VARCHAR(1), "FlWork" VARCHAR(1),
  "Au" VARCHAR(8), "modified_cd" VARCHAR(8),
  "PKey" VARCHAR(16) NOT NULL, "HKey" VARCHAR(64),
  "BusinessModified" TIMESTAMP_NTZ(6), "BusinessAu" VARCHAR(8),
  "HostSource" VARCHAR(50), "Client" VARCHAR(3),
  "Phone1" VARCHAR(30), "Phone2" VARCHAR(30), "Phone3" VARCHAR(30),
  "Fax1" VARCHAR(30), "Fax2" VARCHAR(30), "Fax3" VARCHAR(30),
  "EMail1" VARCHAR(60), "EMail2" VARCHAR(60), "EMail3" VARCHAR(60),
  "URL1" VARCHAR(256), "URL2" VARCHAR(256), "URL3" VARCHAR(256),
  "Id" VARCHAR(50), "Name" VARCHAR(256), "Matchcode" VARCHAR(256),
  "BpaType" VARCHAR(3), "SalesRelevant" VARCHAR(1),
  "LanguageSpoken" VARCHAR(2), "Taxable" VARCHAR(1),
  "TaxJurisdictionCode" VARCHAR(15), "DataSource" VARCHAR(30),
  "Phase" VARCHAR(3), "BpaState" VARCHAR(3),
  "BpaMetaPKey" VARCHAR(16), "BpaMetaHKey" VARCHAR(64),
  "Deleted" VARCHAR(1), "OneTimeCustomer" VARCHAR(1),
  "DeleteReservationDate" TIMESTAMP_NTZ(6),
  "MyPriceListSAP" VARCHAR(10), "MyCustomerType" VARCHAR(1),
  "MySecondaryContact" VARCHAR(30), "MyPriceList" NUMBER(9,0),
  "MyGeoCode" VARCHAR(4), "MyOrgCode1" VARCHAR(4), "MyOrgCode2" VARCHAR(4),
  "MyLiquorLicenseNo" VARCHAR(30), "MyHoldOrderIndicator" VARCHAR(1),
  "MyIsUrgent" VARCHAR(1), "MyCreateNow" VARCHAR(1),
  "MyOrderTakingCustomer" VARCHAR(1),
  "MyTaxClass1" VARCHAR(1), "MyTaxClass2" VARCHAR(1),
  "MyTaxClass3" VARCHAR(1), "MyTaxClass4" VARCHAR(1), "MyTaxClass5" VARCHAR(1),
  "MyConsumerTradeChannel" VARCHAR(4), "MyConsumerSubTradeChannel" VARCHAR(4),
  "MyDTC" VARCHAR(3), "MyTradingChain" VARCHAR(30),
  "MyRedScore" NUMBER(15,6), "MyCAC" VARCHAR(4),
  "MyTradeChannel" VARCHAR(3), "MySubTradeChannel" VARCHAR(100),
  "MySegment" VARCHAR(3), "MyPrimaryContact" VARCHAR(30),
  "MyApprovalStatus" VARCHAR(10), "MyDeliveryRecipient" VARCHAR(1),
  "MyCustomerLocation" VARCHAR(4), "MyMATPhysicalCases" NUMBER(9,0),
  "MyTradingChainInterfaced" VARCHAR(30),
  "MyRedSurveyDate" TIMESTAMP_NTZ(6), "MyRedIndicator" VARCHAR(1),
  "MyIsREDAudited" VARCHAR(1),
  "MySurveyorScore" NUMBER(15,6), "MySelfAssessmentScore" NUMBER(15,6),
  "MySelfAssessmentDate" TIMESTAMP_NTZ(6),
  "MyLastShelfShareDate" TIMESTAMP_NTZ(6), "MyLastShelfShareValue" NUMBER(15,6),
  "MyLastFridgeShareDate" TIMESTAMP_NTZ(6), "MyLastFridgeShareValue" NUMBER(15,6),
  "MyOperationalTradeChannel" VARCHAR(3), "MyOperationalMarketType" VARCHAR(2),
  "MyIsSMO" VARCHAR(1), "MyCreatedManuallyInCAS" VARCHAR(1),
  "MySuperTradeChannel" VARCHAR(30), "MySalesChannel" VARCHAR(4),
  "MyTerritoryID1" VARCHAR(30), "MyTerritoryID2" VARCHAR(30),
  "MyCallStartTime" TIMESTAMP_NTZ(6), "MyDerived" VARCHAR(1)
);

CREATE TABLE "sss1" (
  "extract_source_log_sys" VARCHAR(10), "orig_record_number" VARCHAR(18),
  "rownum" NUMBER(38,0), "subrec" VARCHAR(14),
  "new_record_number" VARCHAR(18), "age_bucket" VARCHAR(1),
  "age_in_days" NUMBER(38,0), "applicant" VARCHAR(25),
  "base_unit_of_measure_type" VARCHAR(3), "company_code" VARCHAR(4),
  "currency_code" VARCHAR(5), "customer_code_2digit" VARCHAR(2),
  "customer_group" VARCHAR(2), "customer_po_number" VARCHAR(35),
  "document_date_vbak" DATE, "document_line_number" VARCHAR(3),
  "document_number" VARCHAR(10), "document_type" VARCHAR(2),
  "dw_customer_code" VARCHAR(20), "dw_customer_type" VARCHAR(20),
  "old_product_code" VARCHAR(20), "old_product_type" VARCHAR(20),
  "new_product_code" VARCHAR(20), "new_product_type" VARCHAR(20),
  "final_bill_flag" VARCHAR(1), "fiscal_year" VARCHAR(4),
  "forecast_finish_date" DATE, "forecast_start_date" DATE,
  "formatted_wbs_element" VARCHAR(30), "gl_account_number" VARCHAR(10),
  "rate" FLOAT,
  "new_group_currency_amt" NUMBER(18,3), "orig_group_currency_amt" NUMBER(18,3),
  "hrc_code" VARCHAR(1), "installation_complete_flag" VARCHAR(1),
  "inventory_location" VARCHAR(35), "investment_category" VARCHAR(1),
  "item_committed_on_job_date" DATE, "item_delivery_block" VARCHAR(2),
  "orig_local_currency_amt" NUMBER(18,3), "new_local_currency_amt" NUMBER(18,3),
  "material_acct_assign_group" VARCHAR(2), "material_code" VARCHAR(18),
  "material_description" VARCHAR(40), "material_document_item" VARCHAR(4),
  "material_document_number" VARCHAR(10), "material_document_year" VARCHAR(4),
  "material_to_cust_cmpl_date" DATE, "material_to_lsc_cmpl_date" DATE,
  "merchandise_class" VARCHAR(18), "movement_type" VARCHAR(3),
  "order_actual_eng_cmpl_date" DATE, "order_actual_eng_start_date" DATE,
  "order_actual_inst_cmpl_date" DATE, "order_actual_inst_start_date" DATE,
  "order_change_note" VARCHAR(100),
  "order_cust_request_cmpl_dt" DATE, "order_cust_request_onjob_dt" DATE,
  "order_cust_request_ship_dt" DATE, "order_delivery_block" VARCHAR(2),
  "order_final_bill_date" DATE, "order_item_note" VARCHAR(100),
  "order_main_ship_date" DATE,
  "order_sched_eng_cmpl_date" DATE, "order_sched_eng_start_date" DATE,
  "order_sched_inst_cmpl_date" DATE, "order_sched_inst_start_date" DATE,
  "order_sched_onjob_date" DATE, "order_ship_complete_date" DATE,
  "orig_fiscal_year" VARCHAR(4), "orig_posting_period" VARCHAR(2),
  "planned_end_date" DATE, "planned_start_date" DATE,
  "plant_code" VARCHAR(4),
  "po_document_number" VARCHAR(10), "po_line_number" VARCHAR(3),
  "poc_flag" VARCHAR(1), "posting_date" DATE, "posting_period" VARCHAR(2),
  "profile_identifier" VARCHAR(7), "profit_center_code" VARCHAR(10),
  "program_field" VARCHAR(20),
  "project_category" VARCHAR(20), "project_category_key" VARCHAR(3),
  "project_group" VARCHAR(10),
  "project_manager_id" VARCHAR(8), "project_manager_name" VARCHAR(25),
  "rejection_reason" VARCHAR(2), "requested_delvry_date" DATE,
  "ret_sales_order_item_number" VARCHAR(6), "ret_sales_order_number" VARCHAR(10),
  "sales_comments" VARCHAR(100),
  "sales_document_item_number" VARCHAR(6), "sales_document_number" VARCHAR(10),
  "sales_document_type" VARCHAR(4), "sales_organization_code" VARCHAR(4),
  "ship_complete_flag" VARCHAR(1),
  "sold_to_customer_number" VARCHAR(10), "storage_location" VARCHAR(4),
  "system_code" VARCHAR(2),
  "orig_total_movement_qty" NUMBER(18,3), "new_total_movement_qty" NUMBER(18,3),
  "trading_partner_id" VARCHAR(6),
  "vendor_account_number" VARCHAR(10), "vendor_name" VARCHAR(35),
  "wbs_element_number" VARCHAR(8), "wbs_id" VARCHAR(10),
  "wbs_status" VARCHAR(4), "wbs_status_desc" VARCHAR(30),
  "wbs_status_type" VARCHAR(1),
  "datetime_entered" TIMESTAMP_NTZ(6)
);

-- =============================================================================
-- Tables needed for SHOW TABLES / SHOW OBJECTS queries
-- =============================================================================

CREATE TABLE "ren_test_t_1_1" ("id" NUMBER(38,0));
CREATE TABLE "ren_test_t_1_4" ("id" NUMBER(38,0));
CREATE TABLE "dtm331_src" ("id" NUMBER(38,0));
CREATE TABLE "dtm386_ambig_tbl" ("id" NUMBER(38,0));
CREATE TABLE "x4" ("id" NUMBER(38,0));

-- =============================================================================
-- Datometry metadata store schema
-- (referenced by __DTM_MDSTORE.__DTM_MDSTORE_TABLE queries)
-- =============================================================================

DROP SCHEMA IF EXISTS "__DTM_MDSTORE" CASCADE;
CREATE SCHEMA "__DTM_MDSTORE";
CREATE TABLE "__DTM_MDSTORE"."__DTM_MDSTORE_TABLE" (
  "SCHEMANAME" VARCHAR(256),
  "OBJECTNAME" VARCHAR(256),
  "COLNAME" VARCHAR(256),
  "PROPNAME" VARCHAR(256),
  "PROPVALUE" VARCHAR(16777216),
  "SEQNO" NUMBER(38,0)
);
