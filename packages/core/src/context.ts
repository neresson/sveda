export interface SvedaContextReadable {
  description: string;
  value: unknown | (() => unknown);
}

export class SvedaContextRegistry {
  private readables = new Map<string, SvedaContextReadable>();

  register(description: string, value: unknown | (() => unknown)): () => void {
    const readable: SvedaContextReadable = { description, value };
    this.readables.set(description, readable);

    return () => this.unregister(description);
  }

  unregister(description: string): void {
    this.readables.delete(description);
  }

  snapshot(): Record<string, unknown> {
    const context: Record<string, unknown> = {};

    for (const readable of this.readables.values()) {
      const value =
        typeof readable.value === 'function'
          ? (readable.value as () => unknown)()
          : readable.value;

      context[readable.description] = value;
    }

    return context;
  }

  clear(): void {
    this.readables.clear();
  }
}
