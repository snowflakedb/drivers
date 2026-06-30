-- Universal Driver — production test account bootstrap
-- Run once per new account as drivers_admin (ACCOUNTADMIN or SECURITYADMIN).
-- Idempotent: all statements use IF NOT EXISTS.
-- Run via: ci/run_prod_setup.sh aws|azure|gcp
-- ---------------------------------------------------------------------------
-- User & role
-- ---------------------------------------------------------------------------
CREATE ROLE IF NOT EXISTS testrole_universal;

-- Test user — TYPE = SERVICE: exempt from MFA enforcement, supports RSA key auth
-- and PATs. Passwords are not supported (nor needed) for SERVICE users.
-- Password-auth E2E tests skip gracefully when SNOWFLAKE_TEST_PASSWORD is absent
-- (see e.g. UserPasswordTests.java, test_user_password.py).
--
-- Pass the public key body via --variable RSA_PUBLIC_KEY="<base64 body>"
-- On re-runs CREATE USER IF NOT EXISTS is a no-op; ALTER TYPE = SERVICE below
-- migrates any existing PERSON user created before this change.
CREATE USER IF NOT EXISTS test_universal
  TYPE = SERVICE
  RSA_PUBLIC_KEY = '&RSA_PUBLIC_KEY'
  DEFAULT_ROLE = testrole_universal
  DEFAULT_WAREHOUSE = testwh_universal;

-- Idempotent migration: no-op when already SERVICE; converts PERSON → SERVICE
-- on accounts provisioned before this script was updated.
ALTER USER test_universal SET TYPE = SERVICE;

GRANT ROLE testrole_universal TO USER test_universal;

-- ---------------------------------------------------------------------------
-- Warehouses
-- ---------------------------------------------------------------------------
CREATE WAREHOUSE IF NOT EXISTS testwh_universal        WITH WAREHOUSE_SIZE = MEDIUM GENERATION = '2' AUTO_SUSPEND = 600 AUTO_RESUME = TRUE;
CREATE WAREHOUSE IF NOT EXISTS testwh_universal_core   WITH WAREHOUSE_SIZE = MEDIUM GENERATION = '2' AUTO_SUSPEND = 600 AUTO_RESUME = TRUE;
CREATE WAREHOUSE IF NOT EXISTS testwh_universal_python WITH WAREHOUSE_SIZE = MEDIUM GENERATION = '2' AUTO_SUSPEND = 600 AUTO_RESUME = TRUE;
CREATE WAREHOUSE IF NOT EXISTS testwh_universal_odbc   WITH WAREHOUSE_SIZE = MEDIUM GENERATION = '2' AUTO_SUSPEND = 600 AUTO_RESUME = TRUE;
CREATE WAREHOUSE IF NOT EXISTS testwh_universal_jdbc   WITH WAREHOUSE_SIZE = MEDIUM GENERATION = '2' AUTO_SUSPEND = 600 AUTO_RESUME = TRUE;

GRANT USAGE ON WAREHOUSE testwh_universal        TO ROLE testrole_universal;
GRANT USAGE ON WAREHOUSE testwh_universal_core   TO ROLE testrole_universal;
GRANT USAGE ON WAREHOUSE testwh_universal_python TO ROLE testrole_universal;
GRANT USAGE ON WAREHOUSE testwh_universal_odbc   TO ROLE testrole_universal;
GRANT USAGE ON WAREHOUSE testwh_universal_jdbc   TO ROLE testrole_universal;

-- ---------------------------------------------------------------------------
-- Sample data (shared from Snowflake-provided share — required by some tests)
-- ---------------------------------------------------------------------------
CREATE DATABASE IF NOT EXISTS SNOWFLAKE_SAMPLE_DATA FROM SHARE SFC_SAMPLES.SAMPLE_DATA;
GRANT IMPORTED PRIVILEGES ON DATABASE SNOWFLAKE_SAMPLE_DATA TO ROLE PUBLIC;

-- ---------------------------------------------------------------------------
-- Database & schema
-- ---------------------------------------------------------------------------
CREATE DATABASE IF NOT EXISTS testdb_universal;
GRANT OWNERSHIP ON DATABASE testdb_universal TO ROLE testrole_universal;
GRANT USAGE ON DATABASE testdb_universal TO ROLE testrole_universal;

-- Re-grant CREATE AUTHENTICATION POLICY on the schema to ACCOUNTADMIN so this
-- script stays idempotent: on re-runs the schema may already be owned by
-- testrole_universal (from a previous GRANT OWNERSHIP ON SCHEMA below), in
-- which case ACCOUNTADMIN needs an explicit privilege to create objects there.
-- ACCOUNTADMIN's global MANAGE GRANTS privilege makes this self-grant legal.
GRANT CREATE AUTHENTICATION POLICY ON SCHEMA testdb_universal.public TO ROLE ACCOUNTADMIN;

-- Authentication policy for SERVICE users: only KEYPAIR, OAUTH, and PAT are
-- supported (PASSWORD is not applicable to SERVICE users; MFA_ENROLLMENT and
-- MFA_POLICY are irrelevant since SERVICE users are always exempt from MFA).
CREATE AUTHENTICATION POLICY IF NOT EXISTS testdb_universal.public.PROGRAMMATIC_ACCESS_USER_AUTH
  AUTHENTICATION_METHODS = ('KEYPAIR', 'OAUTH', 'PROGRAMMATIC_ACCESS_TOKEN')
  CLIENT_TYPES = ('ALL')
  SECURITY_INTEGRATIONS = ('ALL')
  PAT_POLICY = (
    DEFAULT_EXPIRY_IN_DAYS = 15
    MAX_EXPIRY_IN_DAYS = 365
    NETWORK_POLICY_EVALUATION = ENFORCED_REQUIRED
    REQUIRE_ROLE_RESTRICTION_FOR_SERVICE_USERS = TRUE
  );

-- Network Policy
CREATE NETWORK POLICY IF NOT EXISTS CI_POLICY
  ALLOWED_IP_LIST = ('0.0.0.0/0');

ALTER USER TEST_UNIVERSAL UNSET AUTHENTICATION POLICY;
ALTER USER TEST_UNIVERSAL SET AUTHENTICATION POLICY testdb_universal.public.PROGRAMMATIC_ACCESS_USER_AUTH;
ALTER USER TEST_UNIVERSAL UNSET NETWORK_POLICY;
ALTER USER TEST_UNIVERSAL SET NETWORK_POLICY = 'CI_POLICY';

-- COPY CURRENT GRANTS preserves any dependent grants (e.g. CREATE AUTHENTICATION POLICY
-- granted to ACCOUNTADMIN above) so the ownership transfer doesn't fail on re-runs.
GRANT OWNERSHIP ON SCHEMA testdb_universal.public TO ROLE testrole_universal COPY CURRENT GRANTS;

-- ---------------------------------------------------------------------------
-- Allow testrole_universal to create databases (required by ODBC setup scripts)
-- ---------------------------------------------------------------------------
GRANT CREATE DATABASE ON ACCOUNT TO ROLE testrole_universal;

-- ---------------------------------------------------------------------------
-- Git repository (required by put_get git repository e2e tests)
-- ---------------------------------------------------------------------------
CREATE API INTEGRATION IF NOT EXISTS ud_test_git_api_integration
  API_PROVIDER = git_https_api
  API_ALLOWED_PREFIXES = ('https://github.com/snowflakedb/')
  ALLOWED_AUTHENTICATION_SECRETS = ()
  ENABLED = true;

GRANT USAGE ON INTEGRATION ud_test_git_api_integration TO ROLE testrole_universal;

CREATE DATABASE IF NOT EXISTS testing_setup;
GRANT USAGE ON DATABASE testing_setup TO ROLE testrole_universal;
GRANT USAGE ON SCHEMA testing_setup.public TO ROLE testrole_universal;

CREATE OR REPLACE GIT REPOSITORY testing_setup.public.ud_test_homebrew_git_repo
  API_INTEGRATION = ud_test_git_api_integration
  ORIGIN = 'https://github.com/snowflakedb/homebrew-snowflake-cli.git';

GRANT READ ON GIT REPOSITORY testing_setup.public.ud_test_homebrew_git_repo TO ROLE testrole_universal;

ALTER GIT REPOSITORY testing_setup.public.ud_test_homebrew_git_repo FETCH;