import { coreFormatSnowflakeDate } from '../core/index.js';

export class SnowflakeDate extends Date {
  readonly #nanoSeconds: number;
  readonly #scale: number;
  readonly #timezone: string | number;
  readonly #format: string;

  constructor(
    epochMillis: number,
    nanoSeconds: number,
    scale: number,
    timezone: string | number,
    format: string,
  ) {
    super(epochMillis);
    this.#nanoSeconds = nanoSeconds;
    this.#scale = scale;
    this.#timezone = timezone;
    this.#format = format;
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
   * - `string` for DATE / TIMESTAMP_NTZ / TIMESTAMP_LTZ (e.g. `'UTC'`, `'America/New_York'`).
   * - `number` for TIMESTAMP_TZ: the offset from UTC in minutes (e.g. `+05:00` -> `300`).
   */
  getTimezone(): string | number {
    return this.#timezone;
  }

  getFormat(): string {
    return this.#format;
  }

  toJSON(): string {
    return coreFormatSnowflakeDate(
      this.#format,
      this.getTime(),
      this.#nanoSeconds,
      this.#scale,
      this.#timezone,
    );
  }
}
