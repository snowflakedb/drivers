export type DeepPartial<T> = T extends readonly unknown[]
  ? T
  : T extends object
    ? { [K in keyof T]?: DeepPartial<T[K]> }
    : T;

/**
 * Returns a copy of `base` with `patch` merged into it.
 * Nested objects are merged key by key; arrays and other values replace the original.
 */
export function deepMerge<T>(base: T, patch: NoInfer<DeepPartial<T>>): T {
  return mergeValues(base, patch) as T;
}

function mergeValues(base: unknown, patch: unknown): unknown {
  if (!isPlainObject(base) || !isPlainObject(patch)) {
    return patch;
  }
  const merged: Record<string, unknown> = { ...base };
  for (const [key, value] of Object.entries(patch)) {
    merged[key] = mergeValues(base[key], value);
  }
  return merged;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
