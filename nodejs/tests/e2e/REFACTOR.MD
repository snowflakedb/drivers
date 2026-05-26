* When we begin implementing Gherkin coverage, reorganize the tests into clearer groups using folders, and split large test files (those with many tests) into smaller files. e.g.
tests/e2e/query-data-types/
  numbers.test.ts         // current query-data-types-for-numbers.test.ts
  strings.test.ts         // VARCHAR/CHAR/STRING/TEXT
  binary.test.ts          // BINARY
  boolean.test.ts         // BOOLEAN
  datetime.test.ts        // DATE, TIME, TIMESTAMP_LTZ/TZ/NTZ
  semi-structured.test.ts // OBJECT, MAP, ARRAY, VARIANT
  null.test.ts            // NULL handling
