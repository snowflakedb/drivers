export interface Column {
  /**
   * Returns the name of this column.
   */
  getName(): string;

  /**
   * Returns the index of this column. The first column has index 0.
   */
  getIndex(): number;

  /**
   * Returns the id of this column. It is the same value as {@link Column.getIndex}.
   */
  getId(): number;

  /**
   * Returns true if this column is nullable.
   */
  isNullable(): boolean;

  /**
   * Returns the scale of this column, or `null` when the column type has no scale.
   */
  getScale(): number | null;

  /**
   * Returns the precision of this column, or `null` when the column type has no precision.
   */
  getPrecision(): number | null;

  /**
   * Returns the type of this column in lower case, as reported by the server.
   *
   * Values returned today:
   * `fixed`, `real`, `decfloat`, `text`, `binary`, `boolean`, `date`, `time`,
   * `timestamp_ltz`, `timestamp_ntz`, `timestamp_tz`, `variant`, `object`,
   * `array`, `vector`, `interval_year_month`, `interval_day_time`.
   *
   * Some types do not have their own value:
   * - MAP and FILE are reported as `object`.
   * - GEOGRAPHY and GEOMETRY are reported as `object`, `text` or `binary`, depending on
   *   `GEOGRAPHY_OUTPUT_FORMAT` / `GEOMETRY_OUTPUT_FORMAT`.
   *
   * The server may report other types in the future.
   */
  getType(): string;

  /**
   * Returns true if this column is type STRING.
   */
  isString(): boolean;

  /**
   * Returns true if this column is type BINARY.
   */
  isBinary(): boolean;

  /**
   * Returns true if this column is type NUMBER.
   */
  isNumber(): boolean;

  /**
   * Returns true if this column is type DECFLOAT.
   */
  isDecfloat(): boolean;

  /**
   * Returns true if this column is type INTERVAL YEAR TO MONTH.
   */
  isIntervalYearMonth(): boolean;

  /**
   * Returns true if this column is type INTERVAL DAY TO SECOND.
   */
  isIntervalDayTime(): boolean;

  /**
   * Returns true if this column is type BOOLEAN.
   */
  isBoolean(): boolean;

  /**
   * Returns true if this column is type DATE.
   */
  isDate(): boolean;

  /**
   * Returns true if this column is type TIME.
   */
  isTime(): boolean;

  /**
   * Returns true if this column is type TIMESTAMP.
   */
  isTimestamp(): boolean;

  /**
   * Returns true if this column is type TIMESTAMP_LTZ.
   */
  isTimestampLtz(): boolean;

  /**
   * Returns true if this column is type TIMESTAMP_NTZ.
   */
  isTimestampNtz(): boolean;

  /**
   * Returns true if this column is type TIMESTAMP_TZ.
   */
  isTimestampTz(): boolean;

  /**
   * Returns true if this column is type VARIANT, OBJECT, ARRAY or MAP.
   */
  isVariant(): boolean;

  /**
   * Returns true if this column is type OBJECT.
   */
  isObject(): boolean;

  /**
   * Returns true if this column is type ARRAY.
   */
  isArray(): boolean;

  /**
   * Returns true if this column is type MAP.
   */
  isMap(): boolean;

  /**
   * Returns true if this column is type VECTOR.
   */
  isVector(): boolean;
}
