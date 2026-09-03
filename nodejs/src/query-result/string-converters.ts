import type { CellConverter } from './types.js';

// TODO: measure building these strings in the bridge instead of here

const NULL_AS_STRING = 'NULL';

export const textAsStringConverter: CellConverter = (value) =>
  value === null ? NULL_AS_STRING : value;

export const booleanAsStringConverter: CellConverter = (value) => {
  if (value === null) {
    return NULL_AS_STRING;
  }
  return value === true ? 'TRUE' : 'FALSE';
};

export const realAsStringConverter: CellConverter = (value) => {
  switch (value) {
    case null:
      return NULL_AS_STRING;
    case Infinity:
      return 'inf';
    case -Infinity:
      return '-inf';
    default:
      return String(value);
  }
};

// TODO: honor BINARY_OUTPUT_FORMAT=BASE64 once session parameters are read from
// the server response; hex is the default and all that is reachable today.
export const binaryAsStringConverter: CellConverter = (value) =>
  value === null ? NULL_AS_STRING : (value as Buffer).toString('hex').toUpperCase();

// TODO: honor a non-default DATE_OUTPUT_FORMAT once session parameters are read
// from the server response; YYYY-MM-DD is the default and all that is reachable
// today.
export const dateAsStringConverter: CellConverter = (value) =>
  value === null ? NULL_AS_STRING : (value as Date).toISOString().slice(0, 'YYYY-MM-DD'.length);
