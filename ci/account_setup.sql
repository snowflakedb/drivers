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
--   ACCOUNTADMIN (or equivalent) to create API integrations and git repositories

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
