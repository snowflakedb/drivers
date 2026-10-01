import { type CoreDateFormatterInstance } from '../core/index.js';

interface SnowflakeDateConfig {
  value: number | Date;
  nanoSeconds: number;
  scale: number;
  timezone: string | number;
  format: string;
  formatter: CoreDateFormatterInstance;
}

// TODO: for better performance, nodejs_bridge should return this rather than Node constructing it
export class SnowflakeDate extends Date {
  readonly #nanoSeconds: number;
  readonly #scale: number;
  readonly #timezone: string | number;
  readonly #format: string;
  readonly #formatter: CoreDateFormatterInstance;

  constructor(config: SnowflakeDateConfig) {
    super(config.value);
    this.#nanoSeconds = config.nanoSeconds;
    this.#scale = config.scale;
    this.#timezone = config.timezone;
    this.#format = config.format;
    this.#formatter = config.formatter;
  }

  getEpochSeconds(): number {
    return Math.floor(this.getTime() / 1000);
  }

  getNanoSeconds(): number {
    return this.#nanoSeconds;
  }

  getScale(): number {
    return this.#scale;
  }

  /**
   * Timezone of the value:
   * - `string` for TIMESTAMP_NTZ / TIMESTAMP_LTZ (e.g. `'UTC'`, `'America/New_York'`).
   * - `number` for TIMESTAMP_TZ: the offset from UTC in minutes (e.g. `+05:00` -> `300`).
   */
  getTimezone(): string | number {
    return this.#timezone;
  }

  getFormat(): string {
    return this.#format;
  }

  toJSON(): string {
    return this.#formatter.formatTimestamp(
      this.getTime(),
      this.#nanoSeconds,
      this.#scale,
      this.#timezone,
    );
  }
}
