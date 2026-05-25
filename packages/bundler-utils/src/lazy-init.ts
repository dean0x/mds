/**
 * Single-init lazy value holder with dedup and retry semantics.
 *
 * - Concurrent `get()` calls share the in-flight promise — factory is invoked once.
 * - On factory rejection the pending promise is cleared so the next `get()` retries.
 * - Uses a `resolved` boolean flag (not `instance !== undefined`) so `T = void`
 *   and `T = null` work correctly.
 */
export class LazyInit<T> {
  private resolved = false;
  private instance: T | undefined = undefined;
  private pending: Promise<T> | null = null;

  constructor(private readonly factory: () => Promise<T>) {}

  get(): Promise<T> {
    if (this.resolved) return Promise.resolve(this.instance as T);
    if (this.pending === null) {
      this.pending = this.factory().then(
        (result) => {
          this.resolved = true;
          this.instance = result;
          return result;
        },
        (err: unknown) => {
          this.pending = null;
          throw err;
        },
      );
    }
    return this.pending;
  }

  reset(): void {
    this.resolved = false;
    this.instance = undefined;
    this.pending = null;
  }
}
