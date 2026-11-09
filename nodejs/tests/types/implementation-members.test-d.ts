import { describe, expectTypeOf, it } from 'vitest';
import type { ConnectionImpl } from '../../src/connection.js';
import type { CoreColumnInstance } from '../../src/core/index.js';
import type { Column, Connection, RowStatement } from '../../src/public-types/index.js';
import type { RowStatementImpl } from '../../src/query-result/RowStatement.js';

// `implements` fails when a class misses an interface member, but it allows extra members.
// A public method that is not in the interface is not visible to users.
describe('Public interfaces', () => {
  it('should declare every public member of ConnectionImpl', () => {
    expectTypeOf<keyof ConnectionImpl>().toEqualTypeOf<keyof Connection>();
  });

  it('should declare every public member of RowStatementImpl', () => {
    expectTypeOf<keyof RowStatementImpl>().toEqualTypeOf<keyof RowStatement>();
  });

  it('should declare every public member of the native column', () => {
    expectTypeOf<keyof CoreColumnInstance>().toEqualTypeOf<keyof Column>();
  });
});
