import { describe, expect, it } from 'vitest';
import { SvedaToolRegistry } from '../src/tools.js';

describe('SvedaToolRegistry', () => {
  it('registers and lists tool definitions without handlers', () => {
    const registry = new SvedaToolRegistry();
    registry.register({
      name: 'confirm_action',
      description: 'Ask user to confirm',
      parameters: { type: 'object', properties: { title: { type: 'string' } } },
      handler: () => ({ confirmed: true }),
    });

    expect(registry.list()).toEqual([
      {
        name: 'confirm_action',
        description: 'Ask user to confirm',
        parameters: { type: 'object', properties: { title: { type: 'string' } } },
      },
    ]);
  });

  it('executes handlers with input and context', async () => {
    const registry = new SvedaToolRegistry();
    registry.register({
      name: 'navigate',
      description: 'Navigate to a page',
      parameters: {},
      handler: (input, context) => ({ url: input.url, chatId: context.chatId }),
    });

    const result = await registry.execute(
      'navigate',
      { url: '/courses' },
      { chatId: 'c1', toolCallId: 'tc1' }
    );

    expect(result).toEqual({ url: '/courses', chatId: 'c1' });
  });

  it('supports async handlers', async () => {
    const registry = new SvedaToolRegistry();
    registry.register({
      name: 'slow',
      description: 'Slow tool',
      parameters: {},
      handler: async () => {
        await new Promise(resolve => setTimeout(resolve, 5));
        return 42;
      },
    });

    await expect(
      registry.execute('slow', {}, { chatId: 'c', toolCallId: 't' })
    ).resolves.toBe(42);
  });

  it('throws for unknown tools', async () => {
    const registry = new SvedaToolRegistry();
    await expect(
      registry.execute('missing', {}, { chatId: 'c', toolCallId: 't' })
    ).rejects.toThrow('Unknown frontend tool: missing');
  });

  it('unregisters via returned disposer', () => {
    const registry = new SvedaToolRegistry();
    const dispose = registry.register({
      name: 'temp',
      description: 'Temporary',
      parameters: {},
      handler: () => null,
    });

    expect(registry.has('temp')).toBe(true);
    dispose();
    expect(registry.has('temp')).toBe(false);
    expect(registry.list()).toEqual([]);
  });

  it('publishes confirmation only when the tool requires it', () => {
    const registry = new SvedaToolRegistry();
    registry.register({
      name: 'delete_post',
      description: 'Delete a post',
      parameters: {},
      confirmation: 'required',
      handler: () => null,
    });
    registry.register({
      name: 'search_posts',
      description: 'Search posts',
      parameters: {},
      confirmation: 'auto',
      handler: () => null,
    });

    expect(registry.list()).toEqual([
      {
        name: 'delete_post',
        description: 'Delete a post',
        parameters: {},
        confirmation: 'required',
      },
      {
        name: 'search_posts',
        description: 'Search posts',
        parameters: {},
      },
    ]);
  });

  it('filters listed tools by restricted capabilities', () => {
    const registry = new SvedaToolRegistry();
    registry.register({
      name: 'allowed_tool',
      description: 'ok',
      parameters: {},
      handler: () => null,
    });
    registry.register({
      name: 'blocked_tool',
      description: 'no',
      parameters: {},
      handler: () => null,
    });

    registry.setCapabilities({
      restricted: true,
      client: { allow: ['allowed_tool'] },
    });

    expect(registry.list().map(tool => tool.name)).toEqual(['allowed_tool']);
    expect(registry.has('blocked_tool')).toBe(true);
  });
});
