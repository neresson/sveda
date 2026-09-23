import {
  clientToolAllowed,
  type SvedaCapabilities,
  type SvedaClientToolDefinition,
} from '@sveda-ai/protocol';

export interface SvedaFrontendToolContext {
  chatId: string;
  toolCallId: string;
}

export interface SvedaFrontendTool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  confirmation?: 'required' | 'auto';
  handler: (
    input: Record<string, unknown>,
    context: SvedaFrontendToolContext
  ) => unknown | Promise<unknown>;
}

export class SvedaToolRegistry {
  private tools = new Map<string, SvedaFrontendTool>();

  private capabilities: SvedaCapabilities | null = null;

  setCapabilities(capabilities: SvedaCapabilities | null): void {
    this.capabilities = capabilities;
  }

  getCapabilities(): SvedaCapabilities | null {
    return this.capabilities;
  }

  register(tool: SvedaFrontendTool): () => void {
    this.tools.set(tool.name, tool);

    return () => this.unregister(tool.name);
  }

  unregister(name: string): void {
    this.tools.delete(name);
  }

  has(name: string): boolean {
    return this.tools.has(name);
  }

  get(name: string): SvedaFrontendTool | undefined {
    return this.tools.get(name);
  }

  list(): SvedaClientToolDefinition[] {
    return [...this.tools.values()]
      .filter((tool) => clientToolAllowed(tool.name, this.capabilities))
      .map(tool => ({
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
        ...(tool.confirmation === 'required' ? { confirmation: 'required' as const } : {}),
      }));
  }

  async execute(
    name: string,
    input: Record<string, unknown>,
    context: SvedaFrontendToolContext
  ): Promise<unknown> {
    const tool = this.tools.get(name);
    if (!tool) {
      throw new Error(`[sveda] Unknown frontend tool: ${name}`);
    }

    return tool.handler(input, context);
  }

  clear(): void {
    this.tools.clear();
  }
}
