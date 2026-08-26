import { describe, it, expect } from 'vitest';
import ErrorCode from '../../src/constants/ErrorCode.js';
import ErrorMessage from '../../src/constants/ErrorMessage.js';

// TypeScript numeric enums include reverse mappings (e.g. `400001 -> "ERR_..."`).
// Filter to entries whose key is the symbolic name (its value is a number).
// const errorNames = Object.keys(ErrorCode).filter(
//   (value) => !(parseInt(value) > 0),
// ) as (keyof typeof ErrorCode)[];
const errorEntries = Object.entries(ErrorCode).filter(([, value]) => typeof value === 'number') as [
  string,
  number,
][];

describe('Error Codes', () => {
  it.each(errorEntries)('has an ErrorMessage entry for %s (%i)', (_name, code) => {
    expect(ErrorMessage[code], `missing ErrorMessage for code ${code}`).toBeTypeOf('string');
  });

  it('assigns a unique numeric value to every ErrorCode enum member', () => {
    const checkedErrorCodes: Record<number, string> = {};
    for (const [name, code] of errorEntries) {
      expect(checkedErrorCodes[code], `more than one error name for code: ${code}`).toBeUndefined();
      checkedErrorCodes[code] = name;
    }
  });
});
