import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync } from 'node:fs';
import { request as httpRequest } from 'node:http';
import { join } from 'node:path';
import type { ConnectionOptions } from '../../../types/sdk-types.js';
import type { StubMapping } from './stubs.js';
import { findFreePort, sleepAsync } from '../index.js';

const WIREMOCK_VERSION = '3.13.2';
const WIREMOCK_JAR = join(
  import.meta.dirname,
  '..',
  '..',
  '..',
  '..',
  '..',
  'tests',
  'wiremock',
  'wiremock_standalone',
  `wiremock-standalone-${WIREMOCK_VERSION}.jar`,
);

interface LoggedRequest {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: string;
}

export class WiremockServer {
  readonly #host = 'localhost';
  #process?: ChildProcess;
  #port?: number;
  #launchError?: Error;

  // Spawns the shared WireMock standalone JAR over plain HTTP and resolves once
  // it is healthy and ready to serve.
  static async spawn(): Promise<WiremockServer> {
    const server = new WiremockServer();
    await server.#spawn();
    return server;
  }

  get connectionOptions(): ConnectionOptions {
    return {
      host: this.#host,
      port: String(this.#requirePort()),
      protocol: 'http',
    };
  }

  async stub(mapping: StubMapping): Promise<void> {
    const { status, text } = await this.#admin('POST', '/__admin/mappings', mapping);
    if (status !== 200 && status !== 201) {
      throw new Error(`Failed to register WireMock stub: ${status} ${text}`);
    }
  }

  async reset(): Promise<void> {
    const { status, text } = await this.#admin('POST', '/__admin/reset');
    if (status !== 200 && status !== 201) {
      throw new Error(`Failed to reset WireMock: ${status} ${text}`);
    }
  }

  async findRequests(urlPathPattern: string): Promise<LoggedRequest[]> {
    const { status, text } = await this.#admin('POST', '/__admin/requests/find', {
      urlPathPattern,
    });
    if (status !== 200) {
      throw new Error(`Failed to query WireMock requests: ${status} ${text}`);
    }
    const payload = JSON.parse(text) as { requests?: LoggedRequest[] };
    return payload.requests ?? [];
  }

  async destroy(): Promise<void> {
    const child = this.#process;
    this.#process = undefined;
    this.#port = undefined;
    if (!child || child.exitCode !== null) {
      return;
    }
    await new Promise<void>((resolve) => {
      const forceKill = setTimeout(() => {
        if (child.exitCode === null) {
          child.kill('SIGKILL');
        }
      }, 5000);
      forceKill.unref();
      child.once('exit', () => {
        clearTimeout(forceKill);
        resolve();
      });
      child.kill();
    });
  }

  async #spawn(): Promise<void> {
    if (!existsSync(WIREMOCK_JAR)) {
      throw new Error(`WireMock standalone JAR not found at ${WIREMOCK_JAR}`);
    }
    this.#port = await findFreePort();
    const child = spawn(
      'java',
      ['-jar', WIREMOCK_JAR, '--port', String(this.#port), '--disable-banner'],
      { stdio: 'ignore' },
    );
    this.#process = child;
    child.once('error', (error) => {
      this.#launchError = error;
    });
    try {
      await this.#waitForHealth();
    } catch (error) {
      await this.destroy();
      throw error;
    }
  }

  #url(): string {
    return `http://${this.#host}:${this.#requirePort()}`;
  }

  #requirePort(): number {
    if (this.#port === undefined) {
      throw new Error('WiremockServer is not started; use WiremockServer.start()');
    }
    return this.#port;
  }

  #admin(method: string, path: string, body?: unknown): Promise<{ status: number; text: string }> {
    const target = new URL(`${this.#url()}${path}`);
    const payload = body === undefined ? undefined : JSON.stringify(body);
    return new Promise((resolve, reject) => {
      const req = httpRequest(
        target,
        {
          method,
          headers: payload === undefined ? {} : { 'Content-Type': 'application/json' },
        },
        (res) => {
          const chunks: Buffer[] = [];
          res.on('data', (chunk: Buffer) => chunks.push(chunk));
          res.on('end', () => {
            resolve({ status: res.statusCode ?? 0, text: Buffer.concat(chunks).toString('utf8') });
          });
        },
      );
      req.once('error', reject);
      if (payload !== undefined) {
        req.write(payload);
      }
      req.end();
    });
  }

  async #waitForHealth(timeoutMs = 30_000, intervalMs = 250): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    let lastError: unknown;
    while (Date.now() < deadline) {
      if (this.#launchError) {
        throw new Error(
          `Failed to launch WireMock (is Java 11+ on PATH?): ${this.#launchError.message}`,
          { cause: this.#launchError },
        );
      }
      if (this.#process && this.#process.exitCode !== null) {
        throw new Error(`WireMock process exited early with code ${this.#process.exitCode}`);
      }
      try {
        const { status, text } = await this.#admin('GET', '/__admin/health');
        if (status === 200 && text.includes('"healthy"')) {
          return;
        }
      } catch (error) {
        lastError = error;
      }
      await sleepAsync(intervalMs);
    }
    throw new Error(
      `WireMock did not become healthy within ${timeoutMs}ms${lastError ? `: ${String(lastError)}` : ''}`,
    );
  }
}
