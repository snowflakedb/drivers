#!/usr/bin/env groovy

/**
 * Build stages for Rust workspace
 */

def buildWorkspace() {
  return {
    stage('Build') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              cargo build --workspace --all-targets --verbose
            '''
          } else {
            bat '''
              cargo build --workspace --all-targets --verbose
            '''
          }
        }
      }
    }
  }
}

def buildRelease() {
  return {
    stage('Build Release') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              cargo build --workspace --release --verbose
            '''
          } else {
            bat '''
              cargo build --workspace --release --verbose
            '''
          }
        }
      }
    }
  }
}

return this

