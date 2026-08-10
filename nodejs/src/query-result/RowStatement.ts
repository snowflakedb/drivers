import type { CoreStatementInstance } from '../core/index.js';
import type { SnowflakeError } from '../error.js';

export type StatementCallback = (
  err: SnowflakeError | undefined,
  stmt: RowStatement | FileAndStageBindStatement,
  rows: Array<unknown> | undefined,
) => void;

// TODO:
// Not sure if we actually need to have a wrapper around core statement.
// My initial thought is that we might not be able to implement all public
// driver methods from Rust or that using so many FFI calls won't be efficient.
// Refactor if that won't be the case.
export class RowStatement {
  #core: CoreStatementInstance;

  constructor(core: CoreStatementInstance) {
    this.#core = core;
  }

  getNumRows(): number | undefined {
    return this.#core.getNumRows() ?? undefined;
  }

  getQueryId(): string | undefined {
    return this.#core.getQueryId() ?? undefined;
  }

  cancel(callback?: StatementCallback): void {
    this.#core
      .cancel()
      .then(() => callback?.(undefined, this, undefined))
      .catch((err: Error) => callback?.(err as SnowflakeError, this, undefined));
  }
}

export class FileAndStageBindStatement extends RowStatement {
  constructor() {
    super(undefined as unknown as CoreStatementInstance);
    throw new Error('FileAndStageBindStatement is not implemented');
  }
}
