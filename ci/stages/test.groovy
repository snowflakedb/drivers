#!/usr/bin/env groovy

/**
 * Test stages for Rust workspace
 */

def runTests() {
  return {
    stage('Test') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              # Run tests (without integration tests that require Snowflake connection)
              cargo test --workspace --lib --bins --verbose
            '''
          } else {
            bat '''
              REM Run tests (without integration tests that require Snowflake connection)
              cargo test --workspace --lib --bins --verbose
            '''
          }
        }
      }
    }
  }
}

def runIntegrationTests() {
  return {
    stage('Integration Tests') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              # Run all tests including integration tests
              # Requires PARAMETER_PATH environment variable to be set
              cargo test --workspace --verbose
            '''
          } else {
            bat '''
              REM Run all tests including integration tests
              REM Requires PARAMETER_PATH environment variable to be set
              cargo test --workspace --verbose
            '''
          }
        }
      }
    }
  }
}

return this

