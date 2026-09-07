import { describe, it, expect } from 'vitest';
import ErrorCode from '../../src/constants/ErrorCode.js';
import { snowflake } from './utils/index.js';

describe('SDK Constants', () => {
  it('exports ocspModes', () => {
    expect(snowflake.ocspModes).toEqual({
      FAIL_CLOSED: 'FAIL_CLOSED',
      FAIL_OPEN: 'FAIL_OPEN',
      INSECURE: 'INSECURE',
    });
  });

  it('exports ErrrorCode', () => {
    expect(snowflake.ErrorCode).toEqual(ErrorCode);
  });
});
