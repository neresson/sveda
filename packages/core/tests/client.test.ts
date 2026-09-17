import { describe, expect, it, vi } from 'vitest';
import { SvedaClient } from '../src/client.js';
import { messageText, type SvedaDisplayMessage } from '../src/types.js';

function sseResponse(lines: string[]): Response {
  const encoder = new TextEncoder();
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      for (const line of lines) {
        controller.enqueue(encoder.encode(`${line}\n`));
      }
      controller.close();
    },
  });

  return new Response(stream, {
    status: 200,
    headers: { 'Content-Type': 'text/event-stream' },
  });
}

function lastAssistant(sessionMessages: SvedaDisplayMessage[]): SvedaDisplayMessage {
  const message = sessionMessages[sessionMessages.length - 1];
  if (message.role !== 'assistant') {
    throw new Error('last message is not assistant');
  }
  return message;
}

describe('SvedaChatSession', () => {
  it('streams text deltas into the assistant message', async () => {
    const fetchFn = vi.fn(async () =>
      sseResponse([
        'data: {"type":"message.start"}',
        'data: {"type":"text.delta","delta":"Hello"}',
        'data: {"type":"text.delta","delta":" world"}',
        'data: {"type":"message.end","finishReason":"stop"}',
      ])
    ) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-1');

    await session.send('hi');

    expect(session.messages).toHaveLength(2);
    expect(session.messages[0].role).toBe('user');
    expect(messageText(session.messages[0])).toBe('hi');
    expect(messageText(lastAssistant(session.messages))).toBe('Hello world');
    expect(session.status).toBe('idle');
  });

  it('sends context registry snapshot and client tools in the request', async () => {
    let capturedBody = '';
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      capturedBody = String(init.body);
      return sseResponse(['data: {"type":"message.end"}']);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    client.contextRegistry.register('page', { type: 'dashboard' });
    client.toolRegistry.register({
      name: 'confirm',
      description: 'Confirm action',
      parameters: { type: 'object' },
      handler: () => true,
    });

    await client.createSession('chat-ctx').send('hello');

    const body = JSON.parse(capturedBody);
    expect(body.chatId).toBe('chat-ctx');
    expect(body.prompt).toBe('hello');
    expect(body.messages.at(-1)).toMatchObject({ role: 'user', content: 'hello' });
    expect(body.context).toEqual({ page: { type: 'dashboard' } });
    expect(body.clientTools).toEqual([
      { name: 'confirm', description: 'Confirm action', parameters: { type: 'object' } },
    ]);
  });

  it('executes frontend tool calls and auto-submits results', async () => {
    const bodies: string[] = [];
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      bodies.push(String(init.body));
      if (bodies.length === 1) {
        return sseResponse([
          'data: {"type":"tool.call","toolCallId":"tc1","toolName":"confirm","target":"frontend","input":{"title":"Delete?"}}',
          'data: {"type":"message.end"}',
        ]);
      }
      return sseResponse([
        'data: {"type":"text.delta","delta":"Done"}',
        'data: {"type":"message.end"}',
      ]);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    client.toolRegistry.register({
      name: 'confirm',
      description: 'Confirm action',
      parameters: {},
      handler: input => ({ confirmed: true, title: input.title }),
    });

    const session = client.createSession('chat-tools');
    await session.send('delete it');

    await vi.waitFor(() => {
      expect(bodies).toHaveLength(2);
    });

    const secondBody = JSON.parse(bodies[1]);
    expect(secondBody.prompt).toBe('tool_results');
    expect(secondBody.messages.at(-1).role).toBe('tool');
    const toolMessage = secondBody.messages.find(
      (message: { role: string }) => message.role === 'tool'
    );
    expect(toolMessage).toBeDefined();
    expect(toolMessage.parts[0]).toMatchObject({
      type: 'tool-result',
      toolCallId: 'tc1',
      output: { confirmed: true, title: 'Delete?' },
    });
    expect(messageText(lastAssistant(session.messages))).toBe('Done');
  });

  it('emits maxSteps and title events', async () => {
    const fetchFn = vi.fn(async () =>
      sseResponse([
        'data: {"type":"chat.title","title":"New title"}',
        'data: {"type":"max_steps","maxSteps":30,"canContinue":true}',
        'data: {"type":"message.end"}',
      ])
    ) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-events');

    const titleSpy = vi.fn();
    const maxStepsSpy = vi.fn();
    session.on('title', titleSpy);
    session.on('maxSteps', maxStepsSpy);

    await session.send('hi');

    expect(titleSpy).toHaveBeenCalledWith('New title');
    expect(maxStepsSpy).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'max_steps', maxSteps: 30 })
    );
  });

  it('sends protocol-specific accept headers', async () => {
    const seenHeaders: Array<Record<string, string>> = [];
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      seenHeaders.push(init.headers as Record<string, string>);
      return sseResponse(['data: {"type":"message.end"}']);
    }) as unknown as typeof fetch;

    const svedaClient = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    await svedaClient.createSession().send('hi');

    const vercelClient = new SvedaClient({
      endpoints: { stream: '/sveda/stream' },
      protocolMode: 'vercel',
      fetchFn,
    });
    await vercelClient.createSession().send('hi');

    expect(seenHeaders[0]).toMatchObject({ Accept: 'application/vnd.sveda.stream+json' });
    expect(seenHeaders[1]).toMatchObject({
      Accept: 'text/event-stream',
      'X-Sveda-Protocol': 'vercel',
    });
  });

  it('stops streaming on abort without throwing', async () => {
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      const stream = new ReadableStream<Uint8Array>({
        start(controller) {
          init.signal?.addEventListener('abort', () => controller.close());
        },
      });
      return new Response(stream, { status: 200 });
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-abort');

    const promise = session.send('hi');
    session.stop();
    await promise;

    expect(session.status).toBe('idle');
  });

  it('throws on non-ok responses and sets error status', async () => {
    const fetchFn = vi.fn(async () => new Response('nope', { status: 500 })) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-500');
    session.on('error', () => {});

    await expect(session.send('hi')).rejects.toThrow('500');
    expect(session.status).toBe('error');
  });
});

describe('SvedaClient history API', () => {
  it('lists, loads, renames and deletes histories', async () => {
    const calls: Array<{ url: string; method: string }> = [];
    const fetchFn = vi.fn(async (url: string, init?: RequestInit) => {
      const method = init?.method ?? 'GET';
      calls.push({ url: String(url), method });

      if (method === 'GET' && String(url).endsWith('/histories')) {
        return Response.json({ histories: [{ chatId: 'c1', title: 'One' }] });
      }
      if (method === 'GET') {
        return Response.json({
          history: { chatId: 'c1', title: 'One', messages: [] },
        });
      }
      return Response.json({});
    }) as unknown as typeof fetch;

    const client = new SvedaClient({
      endpoints: { stream: '/sveda/stream', histories: '/sveda/histories' },
      fetchFn,
    });

    await expect(client.listHistories()).resolves.toEqual([{ chatId: 'c1', title: 'One' }]);
    await expect(client.getHistory('c1')).resolves.toMatchObject({ chatId: 'c1' });
    await client.renameHistory('c1', 'Renamed');
    await client.deleteHistory('c1');

    expect(calls).toEqual([
      { url: '/sveda/histories', method: 'GET' },
      { url: '/sveda/histories/c1', method: 'GET' },
      { url: '/sveda/histories/c1', method: 'PATCH' },
      { url: '/sveda/histories/c1', method: 'DELETE' },
    ]);
  });

  it('maps history id to chatId when the host omits chatId', async () => {
    const fetchFn = vi.fn(async (url: string, init?: RequestInit) => {
      const method = init?.method ?? 'GET';
      if (method === 'GET' && String(url).endsWith('/histories')) {
        return Response.json({ histories: [{ id: 'c-legacy', title: 'Legacy' }] });
      }

      return Response.json({
        history: { id: 'c-legacy', title: 'Legacy', messages: [] },
      });
    }) as unknown as typeof fetch;

    const client = new SvedaClient({
      endpoints: { stream: '/sveda/stream', histories: '/sveda/histories' },
      fetchFn,
    });

    await expect(client.listHistories()).resolves.toEqual([
      expect.objectContaining({ id: 'c-legacy', chatId: 'c-legacy', title: 'Legacy' }),
    ]);
    await expect(client.getHistory('c-legacy')).resolves.toMatchObject({ chatId: 'c-legacy' });
  });

  it('invokes the default fetch with window as this', async () => {
    const browserFetch = function (this: unknown) {
      if (this !== globalThis) {
        throw new TypeError("Failed to execute 'fetch' on 'Window': Illegal invocation");
      }

      return Promise.resolve(Response.json({ histories: [] }));
    } as typeof fetch;

    vi.stubGlobal('fetch', browserFetch);

    try {
      const client = new SvedaClient({
        endpoints: { stream: '/sveda/stream', histories: '/sveda/histories' },
      });

      await expect(client.listHistories()).resolves.toEqual([]);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
