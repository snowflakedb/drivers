import { onTestFinished, vi } from 'vitest';

export function createConsoleCapture(): string[] {
  const stream = (console as unknown as { _stdout?: NodeJS.WritableStream })._stdout;
  if (!stream) {
    throw new Error('console._stdout is missing');
  }
  const writes: string[] = [];
  const spy = vi.spyOn(stream, 'write').mockImplementation((chunk) => {
    writes.push(String(chunk));
    return true;
  });
  onTestFinished(() => spy.mockRestore());
  return writes;
}
