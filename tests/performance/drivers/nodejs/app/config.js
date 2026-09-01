'use strict';

const fs = require('fs');

// v1: SELECT and COLD_START only. PUT_GET is not implemented for nodejs yet.
// COLD_START has zero setup queries (see conftest.py's _prepare_setup_queries),
// which sidesteps the ARROW-format setup query that SELECT tests need.
const SUPPORTED_TEST_TYPES = new Set(['select', 'cold_start']);

class TestConfig {
  constructor() {
    this.driverType = process.env.DRIVER_TYPE || 'universal';

    const testType = process.env.TEST_TYPE || 'select';
    if (!SUPPORTED_TEST_TYPES.has(testType)) {
      console.error(
        `ERROR: Unsupported test type '${testType}'. Only 'select' is supported for the nodejs driver in v1.`,
      );
      process.exit(1);
    }
    this.testType = testType;

    this.sqlCommand = process.env.SQL_COMMAND;
    this.testName = process.env.TEST_NAME;
    this.iterations = parseInt(process.env.PERF_ITERATIONS || '1', 10);
    this.warmupIterations = parseInt(process.env.PERF_WARMUP_ITERATIONS || '0', 10);
    this.paramsJson = process.env.PARAMETERS_JSON;
    this.setupQueriesJson = process.env.SETUP_QUERIES;

    if (!this.sqlCommand || !this.testName || !this.paramsJson) {
      console.error('ERROR: Missing required environment variables');
      process.exit(1);
    }
  }

  getSetupQueries() {
    if (this.setupQueriesJson) {
      return JSON.parse(this.setupQueriesJson);
    }
    return [];
  }

  parseConnectionParams() {
    const params = JSON.parse(this.paramsJson);
    const connParams = params.testconnection || {};

    const privateKeyFile = connParams.SNOWFLAKE_TEST_PRIVATE_KEY_FILE;
    let privateKey;

    if (privateKeyFile) {
      if (!fs.existsSync(privateKeyFile)) {
        console.error(`ERROR: Private key file '${privateKeyFile}' does not exist`);
        process.exit(1);
      }
      privateKey = fs.readFileSync(privateKeyFile, 'utf8');
    } else {
      const privateKeyContents = connParams.SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS;
      if (privateKeyContents && privateKeyContents.length) {
        privateKey = privateKeyContents.join('\n') + '\n';
      }
    }

    const connectionParams = {
      account: connParams.SNOWFLAKE_TEST_ACCOUNT || connParams.account,
      host: connParams.SNOWFLAKE_TEST_HOST || connParams.host,
      username: connParams.SNOWFLAKE_TEST_USER || connParams.user,
      database: connParams.SNOWFLAKE_TEST_DATABASE || connParams.database,
      schema: connParams.SNOWFLAKE_TEST_SCHEMA || connParams.schema,
      warehouse: connParams.SNOWFLAKE_TEST_WAREHOUSE || connParams.warehouse,
      role: connParams.SNOWFLAKE_TEST_ROLE || connParams.role,
    };

    if (privateKey) {
      connectionParams.authenticator = 'SNOWFLAKE_JWT';
      connectionParams.privateKey = privateKey;
    }

    return connectionParams;
  }

  getDriverLabel() {
    return this.driverType === 'universal' ? 'NODEJS' : 'NODEJS (Old)';
  }
}

module.exports = { TestConfig };
