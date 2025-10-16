#!/usr/bin/env groovy

/**
 * Setup and initialization stages
 */

def setupRust() {
  return {
    stage('Setup Rust') {
      steps {
        script {
          if (isUnix()) {
            sh '''
              # Install or update Rust
              if command -v rustc >/dev/null 2>&1; then
                rustup update stable
              else
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source $HOME/.cargo/env
              fi
              rustc --version
              cargo --version
            '''
          } else {
            bat '''
              @echo off
              rustc --version || (
                echo Installing Rust...
                curl --proto =https --tlsv1.2 -sSf https://win.rustup.rs/x86_64 -o rustup-init.exe
                rustup-init.exe -y
                del rustup-init.exe
              )
              rustc --version
              cargo --version
            '''
          }
        }
      }
    }
  }
}

return this

