import { Connection, ConnectionOptions } from "./connection";
import { configure as configureLogger, ConfigureOptions } from "./logger";

export { Connection, ConnectionOptions } from "./connection";
export { ConfigureOptions, LogLevel } from "./logger";

export function createConnection(options: ConnectionOptions): Connection {
  return new Connection(options);
}

export function configure(options: ConfigureOptions): void {
  configureLogger(options);
}
