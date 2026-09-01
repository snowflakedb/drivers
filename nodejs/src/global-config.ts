import { XMLParser, XMLValidator, X2jOptions } from 'fast-xml-parser';

export type CustomParser = (rawColumnValue: string) => any;
export type XMlParserConfigOption = X2jOptions;

export interface ConfigureOptions {
  /**
   * Custom parser for JSON data in VARIANT, OBJECT, and ARRAY columns.
   *
   * By default the driver parses values with `JSON.parse()`. If that fails (e.g. the
   * value contains non-standard tokens like `undefined`, `NaN`, or `Infinity` that
   * Snowflake's VARIANT type allows), it falls back to eval-based parsing, which is
   * slower and logs a warning.
   *
   * To avoid the fallback, set the `STRICT_JSON_OUTPUT` session parameter to `TRUE` so
   * Snowflake normalizes non-standard values into valid JSON before sending them.
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume
   * @see https://docs.snowflake.com/en/sql-reference/parameters#strict-json-output
   */
  jsonColumnVariantParser?: CustomParser;

  /**
   * Custom parser for XML data in VARIANT columns.
   *
   * The driver always attempts JSON parsing first for every VARIANT value. Only when
   * JSON parsing fails does it try this XML parser, so XML values always incur the
   * overhead of a failed JSON parse attempt before being handled.
   *
   * The built-in parser uses `fast-xml-parser` and ignores XML attributes by default.
   * Use `xmlParserConfig` to customize attribute handling.
   *
   * @see https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-consume
   */
  xmlColumnVariantParser?: CustomParser;

  /**
   * Configuration passed through to `fast-xml-parser`, the library backing the
   * built-in `xmlColumnVariantParser`.
   *
   * These options are forwarded directly to the `XMLParser` constructor, so they
   * control how XML in VARIANT columns is parsed (e.g. attribute handling, tag value
   * processing). See `fast-xml-parser`'s `X2jOptions` for the full set of options.
   *
   * Has no effect if you supply your own `xmlColumnVariantParser`.
   */
  xmlParserConfig?: XMlParserConfigOption;

  /**
   * Whether the driver reads proxy settings from the `HTTP_PROXY`, `HTTPS_PROXY`, and `NO_PROXY`
   * environment variables when a connection does not configure an explicit proxy. An explicit
   * `proxyHost`/`proxyPort` always takes precedence over the environment.
   *
   * @default true
   */
  useEnvProxy?: boolean;
}

type ResolvedConfig = Required<ConfigureOptions>;

const buildDefaults = (): ResolvedConfig => ({
  jsonColumnVariantParser: (
    rawColumnValue: string,
    // NOTE: We're intentionally not documenting the context argument in CustomParser to match old driver typing
    context?: {
      onNonJsonCompliantVariant: () => void;
    },
  ) => {
    try {
      return JSON.parse(rawColumnValue);
    } catch {
      const result = new Function(`return (${rawColumnValue});`)();
      context?.onNonJsonCompliantVariant();
      return result;
    }
  },
  xmlColumnVariantParser: (rawColumnValue: string) => {
    const validateResult = XMLValidator.validate(rawColumnValue);
    if (validateResult === true) {
      return new XMLParser(GlobalConfig.xmlParserConfig).parse(rawColumnValue);
    } else {
      throw new Error(validateResult.err.msg);
    }
  },
  xmlParserConfig: {},
  useEnvProxy: true,
});

export const GlobalConfig: ResolvedConfig = {
  ...buildDefaults(),
};

export function updateGlobalConfig(options: Partial<ConfigureOptions>) {
  Object.assign(GlobalConfig, options);
}

export const resetGlobalConfig = () => {
  Object.assign(GlobalConfig, buildDefaults());
};
