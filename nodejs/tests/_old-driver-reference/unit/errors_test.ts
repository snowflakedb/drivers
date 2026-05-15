import assert from 'assert';
import { ErrorCode } from './../../lib/errors';
import { mapErrorCodeToSqlState } from './../../lib/errors';

describe('Errors', function () {
  it('validate error code to sql state mapping', function () {
    for (const [errCode, sqlState] of Object.entries(mapErrorCodeToSqlState)) {
      assert.ok(errCode in ErrorCode, `invalid mapping: ${errCode}:${sqlState}`);
    }
  });
});
