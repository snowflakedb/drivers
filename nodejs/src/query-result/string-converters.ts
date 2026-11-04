import type { CellConverter } from './types.js';

// TODO: measure building these strings in the bridge instead of here

const NULL_AS_STRING = 'NULL';

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

export const vectorAsStringConverter: CellConverter = (value) =>
  value === null ? NULL_AS_STRING : JSON.stringify(value);
