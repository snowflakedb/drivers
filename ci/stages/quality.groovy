#!/usr/bin/env groovy

/**
 * Code quality and linting stages
 */

def checkFormatting() {
  return {
    stage('Check Formatting') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              rustup component add rustfmt
              cargo fmt --all -- --check
            '''
          } else {
            bat '''
              rustup component add rustfmt
              cargo fmt --all -- --check
            '''
          }
        }
      }
    }
  }
}

def runClippy() {
  return {
    stage('Clippy') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              rustup component add clippy
              cargo clippy --workspace --all-targets -- -D warnings
            '''
          } else {
            bat '''
              rustup component add clippy
              cargo clippy --workspace --all-targets -- -D warnings
            '''
          }
        }
      }
    }
  }
}

def runSecurityAudit() {
  return {
    stage('Security Audit') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              # Install cargo-audit if not present
              cargo install --list | grep cargo-audit || cargo install cargo-audit
              cargo audit
            '''
          } else {
            bat '''
              REM Install cargo-audit if not present
              cargo install --list | findstr cargo-audit || cargo install cargo-audit
              cargo audit
            '''
          }
        }
      }
    }
  }
}

return this

