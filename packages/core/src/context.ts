export interface VedaContextReadable {
  description: string;
  value: unknown | (() => unknown);
}

export class VedaContextRegistry {
  private readables = new Map<string, VedaContextReadable>();

  register(description: string, value: unknown | (() => unknown)): () => void {
    const readable: VedaContextReadable = { description, value };
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
