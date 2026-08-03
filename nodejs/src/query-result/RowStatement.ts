import type { CoreStatementInstance } from '../core/index.js';

// TODO:
// Not sure if we actually need to have a wrapper around core statement.
// My initial thought is that we might not be able to implement all public
// driver methods from Rust or that using so many FFI calls won't be efficient.
// Refactor if that won't be the case.
export class RowStatement {
  #core?: CoreStatementInstance;

  constructor(coreStatement: Promise<CoreStatementInstance>) {
    coreStatement
      .then((core) => {
        this.#core = core;
      })
      .catch(() => {
        // Failure here is no-op
      });
  }

  getNumRows(): number | undefined {
    return this.#core?.getNumRows() ?? undefined;
  }

  getQueryId(): string | undefined {
    return this.#core?.getQueryId() ?? undefined;
  }
}

export class FileAndStageBindStatement extends RowStatement {
  constructor(coreStatement: Promise<CoreStatementInstance>) {
    super(coreStatement);
    throw new Error('FileAndStageBindStatement is not implemented');
  }
}
