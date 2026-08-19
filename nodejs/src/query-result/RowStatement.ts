import type { Readable } from 'node:stream';
import type { CoreStatementInstance } from '../core/index.js';
import type { SnowflakeError } from '../error.js';
import type {
  Column,
  FetchRowsOptions,
  RowMode,
  StatementCallback,
  StreamOptions,
} from './types.js';
import { createRowStream } from './rows.js';

// TODO:
// Not sure if we actually need to have a wrapper around core statement.
// My initial thought is that we might not be able to implement all public
// driver methods from Rust or that using so many FFI calls won't be efficient.
// Refactor if that won't be the case.
export class RowStatement {
  #core: CoreStatementInstance;
  #rowMode: RowMode;

  constructor(core: CoreStatementInstance, rowMode?: RowMode) {
    this.#core = core;
    this.#rowMode = rowMode ?? 'object';
  }

  get rowMode(): RowMode {
    return this.#rowMode;
  }

  getNumRows(): number | undefined {
    return this.#core.getNumRows() ?? undefined;
  }

  getQueryId(): string | undefined {
    return this.#core.getQueryId() ?? undefined;
  }

  getColumns(): Column[] | undefined {
    return this.#core.getColumns() ?? undefined;
  }

  getColumn(columnIdentifier: string | number): Column | undefined {
    return this.#core.getColumn(columnIdentifier) ?? undefined;
  }

  // TODO: decide how to handle a case where user didn't set the streamResult: true
  // and the result is already drained. (would suggest a BCR with error)
  // oxlint-disable-next-line no-unused-vars
  streamRows(options?: StreamOptions): Readable {
    return createRowStream(this.#core, this.#rowMode);
  }

  fetchRows(options: FetchRowsOptions): void {
    const stream = createRowStream(this.#core, this.#rowMode);
    let finished = false;

    const onComplete = (err: SnowflakeError | undefined) => {
      if (finished) {
        return;
      }
      finished = true;
      options.end(err, this);
    };

    stream.on('data', (row: unknown) => {
      if (finished) {
        return;
      }
      if (options.each(row) === false) {
        stream.destroy();
        onComplete(undefined);
      }
    });
    stream.on('end', () => onComplete(undefined));
    stream.on('error', (err: Error) => onComplete(err as SnowflakeError));
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
