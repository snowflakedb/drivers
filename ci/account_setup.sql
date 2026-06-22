-- Universal driver e2e account setup
-- Run once per test account before running the e2e suite.
--
-- Usage (mirrors snowflake-cli/tests_integration/scripts/integration_account_setup.sql):
--   snow sql \
--     -f ci/account_setup.sql \
--     -c <your_connection_name>
--
-- Required privileges to run this script:
--   SECURITYADMIN (or equivalent) to create roles and grant them to users
--   ACCOUNTADMIN (or equivalent) to create API integrations and git repositories,
--     compute pools, and image repositories

CREATE ROLE IF NOT EXISTS testrole_universal;

GRANT ROLE testrole_universal TO USER test_universal;

CREATE API INTEGRATION IF NOT EXISTS ud_test_git_api_integration
  API_PROVIDER = git_https_api
  API_ALLOWED_PREFIXES = ('https://github.com/snowflakedb/')
  ALLOWED_AUTHENTICATION_SECRETS = ()
  ENABLED = true;

GRANT USAGE ON INTEGRATION ud_test_git_api_integration TO ROLE testrole_universal;

CREATE DATABASE IF NOT EXISTS  testing_setup;
GRANT USAGE ON DATABASE testing_setup TO ROLE testrole_universal;
GRANT USAGE ON SCHEMA testing_setup.public TO ROLE testrole_universal;

CREATE OR REPLACE GIT REPOSITORY testing_setup.public.ud_test_homebrew_git_repo
  API_INTEGRATION = ud_test_git_api_integration
  ORIGIN = 'https://github.com/snowflakedb/homebrew-snowflake-cli.git';

GRANT READ ON GIT REPOSITORY testing_setup.public.ud_test_homebrew_git_repo TO ROLE testrole_universal;

ALTER GIT REPOSITORY testing_setup.public.ud_test_homebrew_git_repo FETCH;

-- ---------------------------------------------------------------------------
-- Snowpark Container Services (SPCS) — pre-reqs for the SPCS auth e2e test.
--
-- The SPCS e2e test builds a probe image, pushes it to the image repository
-- below, and runs it as a job service in the compute pool below. Inside the
-- job, the driver authenticates with the platform-injected OAuth token (no
-- user) and the driver attaches the SPCS_TOKEN service identifier, proving the
-- in-SPCS login path end-to-end.
--
-- The pool uses an ARM instance family (GEN_ARM_G1_2): the probe image is
-- built linux/arm64 and CI runs on ARM runners, so the whole path is ARM-native
-- (no cross-arch emulation).
-- ---------------------------------------------------------------------------

CREATE COMPUTE POOL IF NOT EXISTS ud_test_spcs_pool
  MIN_NODES = 1
  MAX_NODES = 1
  INSTANCE_FAMILY = GEN_ARM_G1_2
  AUTO_RESUME = TRUE
  AUTO_SUSPEND_SECS = 300;

GRANT USAGE, MONITOR, OPERATE ON COMPUTE POOL ud_test_spcs_pool TO ROLE testrole_universal;

CREATE IMAGE REPOSITORY IF NOT EXISTS testing_setup.public.ud_test_image_repo;

GRANT READ, WRITE ON IMAGE REPOSITORY testing_setup.public.ud_test_image_repo TO ROLE testrole_universal;

-- Allow the test role to create (and run) the job service in the schema.
GRANT CREATE SERVICE ON SCHEMA testing_setup.public TO ROLE testrole_universal;
