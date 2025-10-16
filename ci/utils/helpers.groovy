#!/usr/bin/env groovy

/**
 * Helper functions for Jenkins pipeline
 */

/**
 * Map platform names to Jenkins agent labels
 */
def getAgentLabel(String platform) {
  switch(platform) {
    case 'linux-x86_64':
      return 'small-node-c7'
    case 'linux-arm64':
      return 'linux-arm64'
    case 'macos-x86_64':
      return 'macos-x86_64'
    case 'macos-arm64':
      return 'macos-arm64'
    case 'windows-x86_64':
      return 'windows-x86_64'
    default:
      return 'small-node-c7'
  }
}

/**
 * Get list of all supported platforms
 */
def getSupportedPlatforms() {
  return ['linux-x86_64', 'linux-arm64', 'macos-x86_64', 'macos-arm64', 'windows-x86_64']
}

/**
 * Check if running on Unix-like system
 */
def isUnixPlatform(String platform) {
  return platform.startsWith('linux-') || platform.startsWith('macos-')
}

return this

