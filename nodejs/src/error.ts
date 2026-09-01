export interface SnowflakeError extends Error {
  code?: string | number;
  sqlState?: string;
}
