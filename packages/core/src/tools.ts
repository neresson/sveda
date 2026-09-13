import type { VedaClientToolDefinition } from '@veda-ai/protocol';

export interface VedaFrontendToolContext {
  chatId: string;
  toolCallId: string;
}

export interface VedaFrontendTool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  handler: (
    input: Record<string, unknown>,
    context: VedaFrontendToolContext
  ) => unknown | Promise<unknown>;
}

export class VedaToolRegistry {
  private tools = new Map<string, VedaFrontendTool>();

  register(tool: VedaFrontendTool): () => void {
    this.tools.set(tool.name, tool);

    return () => this.unregister(tool.name);
  }

  unregister(name: string): void {
    this.tools.delete(name);
  }

  has(name: string): boolean {
    return this.tools.has(name);
  }

  get(name: string): VedaFrontendTool | undefined {
    return this.tools.get(name);
  }

  list(): VedaClientToolDefinition[] {
    return [...this.tools.values()].map(tool => ({
      name: tool.name,
      description: tool.description,
      parameters: tool.parameters,
    }));
  }

  async execute(
    name: string,
    input: Record<string, unknown>,
    context: VedaFrontendToolContext
  ): Promise<unknown> {
    const tool = this.tools.get(name);
    if (!tool) {
      throw new Error(`[veda] Unknown frontend tool: ${name}`);
    }

    return tool.handler(input, context);
  }

  clear(): void {
    this.tools.clear();
  }
}
