import { executeCommand } from '../../scripts/common.mjs';

export default function setup() {
  // Type tests read the built `snowflake-sdk` package. The native core is not needed for types.
  for (const script of ['build:sdk', 'build:link-local']) {
    executeCommand('npm', ['run', script]);
  }
}
