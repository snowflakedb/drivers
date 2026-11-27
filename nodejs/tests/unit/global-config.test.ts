import { describe, it, expect, afterEach } from 'vitest';
import { GlobalConfig, updateGlobalConfig, resetGlobalConfig } from '../../src/global-config.js';
import snowflake from '../../src/index.js';

describe('GlobalConfig', () => {
  afterEach(() => {
    resetGlobalConfig();
  });

  it('reads default config value', () => {
    expect(GlobalConfig.xmlParserConfig).toEqual({});
  });

  it('reads custom config value', () => {
    updateGlobalConfig({ xmlParserConfig: { ignoreAttributes: false } });
    expect(GlobalConfig.xmlParserConfig).toEqual({ ignoreAttributes: false });
  });

  describe('jsonColumnVariantParser', () => {
    it('parses standard JSON values', () => {
      expect(GlobalConfig.jsonColumnVariantParser('{"a":1}')).toEqual({ a: 1 });
      expect(GlobalConfig.jsonColumnVariantParser('[1,2,3]')).toEqual([1, 2, 3]);
      expect(GlobalConfig.jsonColumnVariantParser('"hello"')).toBe('hello');
    });

    it.each(['{"a":undefined}', '{"a":NaN}', '{"a":Infinity}', '[-Infinity]'])(
      'should throw a SyntaxError on the non-JSON value %s',
      (rawColumnValue) => {
        expect(() => GlobalConfig.jsonColumnVariantParser(rawColumnValue)).toThrow(SyntaxError);
      },
    );

    it('should throw a SyntaxError on invalid JSON', () => {
      expect(() => GlobalConfig.jsonColumnVariantParser('{invalid')).toThrow(SyntaxError);
    });
  });

  describe('xmlColumnVariantParser', () => {
    it('parses valid XML using the default config', () => {
      expect(GlobalConfig.xmlColumnVariantParser('<root><a>1</a></root>')).toEqual({
        root: { a: 1 },
      });
    });

    it('throws on invalid XML', () => {
      expect(() => GlobalConfig.xmlColumnVariantParser('<root><a>1</root>')).toThrow();
    });

    it('honors xmlParserConfig when parsing attributes', () => {
      expect(GlobalConfig.xmlColumnVariantParser('<root a="1">text</root>')).toEqual({
        root: 'text',
      });
      updateGlobalConfig({ xmlParserConfig: { ignoreAttributes: false } });
      expect(GlobalConfig.xmlColumnVariantParser('<root a="1">text</root>')).toEqual({
        root: { '@_a': '1', '#text': 'text' },
      });
    });
  });

  describe('customCredentialManager', () => {
    it.each([
      ['a string', 'unsupported', 'read'],
      ['an empty object', {}, 'read'],
      ['a manager missing read', { write() {}, remove() {} }, 'read'],
      ['a manager missing write', { read() {}, remove() {} }, 'write'],
      ['a manager missing remove', { read() {}, write() {} }, 'remove'],
    ])('should reject %s', (_, manager, method) => {
      expect(() => snowflake.configure({ customCredentialManager: manager as never })).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: undefined,
          message: `Object property '${method}' type mismatch. Expect value to be Function, but received Undefined`,
        }),
      );
    });

    it('should accept an object with the required callbacks', () => {
      expect(() =>
        snowflake.configure({
          customCredentialManager: {
            read: () => null,
            write: () => undefined,
            remove: () => undefined,
          },
        }),
      ).not.toThrow();
    });
  });
});
