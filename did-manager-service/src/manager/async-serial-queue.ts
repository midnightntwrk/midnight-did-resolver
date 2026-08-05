export class AsyncSerialQueue {
  private tail: Promise<void> = Promise.resolve();

  async run<T>(operation: () => Promise<T>): Promise<T> {
    const runAfterTail = this.tail.then(operation, operation);
    this.tail = runAfterTail.then(
      () => undefined,
      () => undefined,
    );
    return await runAfterTail;
  }
}
