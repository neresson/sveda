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

  it('adopts chatId and messageId from message.start', async () => {
    const fetchFn = vi.fn(async () =>
      sseResponse([
        'data: {"type":"message.start","chatId":"server-chat","messageId":"m-server"}',
        'data: {"type":"text.delta","delta":"Hi"}',
        'data: {"type":"message.end","finishReason":"stop"}',
      ])
    ) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('local-chat');

    await session.send('hi');

    expect(session.chatId).toBe('server-chat');
    expect(lastAssistant(session.messages).id).toBe('m-server');
  });

  it('forwards thinking options and appends reasoning deltas', async () => {
    let capturedBody = '';
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      capturedBody = String(init.body);
      return sseResponse([
        'data: {"type":"message.start"}',
        'data: {"type":"reasoning.delta","delta":"plan"}',
        'data: {"type":"text.delta","delta":"done"}',
        'data: {"type":"message.end","finishReason":"stop"}',
      ]);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-think');
    await session.send('hi', { options: { thinking: false } });

    expect(JSON.parse(capturedBody).options).toEqual({ thinking: false });
    const assistant = lastAssistant(session.messages);
    expect(assistant.parts).toEqual([
      { type: 'reasoning', text: 'plan' },
      { type: 'text', text: 'done' },
    ]);
  });

  it('replays a reasoning placeholder on assistant turns that never stored CoT', async () => {
    const bodies: unknown[] = [];
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      bodies.push(JSON.parse(String(init.body)));
      return sseResponse([
        'data: {"type":"message.start"}',
        'data: {"type":"text.delta","delta":"hello"}',
        'data: {"type":"message.end","finishReason":"stop"}',
      ]);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-replay');
    await session.send('hi');
    await session.send('try again');

    const second = bodies[1] as { messages: Array<{ role: string; parts: Array<{ type: string; text?: string }> }> };
    const assistant = second.messages.find(message => message.role === 'assistant');
    expect(assistant?.parts[0]).toEqual({ type: 'reasoning', text: '.' });
    expect(assistant?.parts.some(part => part.type === 'text' && part.text === 'hello')).toBe(true);
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

  it('does not run a frontend tool until the user confirms it', async () => {
    const bodies: string[] = [];
    const handler = vi.fn((input: { id?: string }) => ({ deleted: true, id: input.id }));
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      bodies.push(String(init.body));
      if (bodies.length === 1) {
        return sseResponse([
          'data: {"type":"tool.call","toolCallId":"tc1","toolName":"delete_post","target":"frontend","input":{"id":"1"},"confirmation":"required"}',
          'data: {"type":"message.end","finishReason":"tool_calls"}',
        ]);
      }
      return sseResponse([
        'data: {"type":"text.delta","delta":"Deleted"}',
        'data: {"type":"message.end","finishReason":"stop"}',
      ]);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    client.toolRegistry.register({
      name: 'delete_post',
      description: 'Delete a post',
      parameters: {},
      confirmation: 'required',
      handler,
    });

    const session = client.createSession('chat-confirm');
    await session.send('delete it');

    expect(bodies).toHaveLength(1);
    expect(handler).not.toHaveBeenCalled();

    await session.resolveToolConfirmation('tc1', 'approve');

    expect(handler).toHaveBeenCalledWith({ id: '1' }, expect.objectContaining({ toolCallId: 'tc1' }));
    expect(bodies).toHaveLength(2);
    const secondBody = JSON.parse(bodies[1]);
    expect(secondBody.prompt).toBe('');
    expect(secondBody.toolDecisions).toBeUndefined();
    const original = session.messages.find(message =>
      message.parts.some(part => part.type === 'tool-call' && part.toolCallId === 'tc1')
    );
    expect(original?.parts.some(part => part.type === 'tool-result' && part.toolCallId === 'tc1')).toBe(true);
    expect(messageText(lastAssistant(session.messages))).toBe('Deleted');
  });

  it('does not run a frontend tool when the user denies it', async () => {
    const bodies: string[] = [];
    const handler = vi.fn(() => ({ deleted: true }));
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      bodies.push(String(init.body));
      if (bodies.length === 1) {
        return sseResponse([
          'data: {"type":"tool.call","toolCallId":"tc1","toolName":"delete_post","target":"frontend","confirmation":"required","input":{}}',
          'data: {"type":"message.end","finishReason":"tool_calls"}',
        ]);
      }
      return sseResponse(['data: {"type":"message.end","finishReason":"stop"}']);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    client.toolRegistry.register({
      name: 'delete_post',
      description: 'Delete a post',
      parameters: {},
      confirmation: 'required',
      handler,
    });

    const session = client.createSession('chat-deny');
    await session.send('delete it');
    await session.resolveToolConfirmation('tc1', 'deny');

    expect(handler).not.toHaveBeenCalled();
    const secondBody = JSON.parse(bodies[1]);
    const toolResult = secondBody.messages
      .flatMap((message: { parts?: Array<{ type: string; output?: { denied?: boolean } }> }) => message.parts ?? [])
      .find((part: { type: string }) => part.type === 'tool-result');
    expect(toolResult.output).toMatchObject({ success: false, denied: true });
  });

  it('sends backend confirmation decisions without client arguments', async () => {
    const bodies: string[] = [];
    const fetchFn = vi.fn(async (_url: string, init: RequestInit) => {
      bodies.push(String(init.body));
      if (bodies.length === 1) {
        return sseResponse([
          'data: {"type":"tool.call","toolCallId":"tc1","toolName":"delete_post","target":"backend","input":{"id":"1"},"confirmation":"required"}',
          'data: {"type":"message.end","finishReason":"tool_calls"}',
        ]);
      }
      return sseResponse([
        'data: {"type":"tool.result","toolCallId":"tc1","toolName":"delete_post","output":{"success":true}}',
        'data: {"type":"message.start","messageId":"m2"}',
        'data: {"type":"text.delta","delta":"Removed"}',
        'data: {"type":"message.end","finishReason":"stop"}',
      ]);
    }) as unknown as typeof fetch;

    const client = new SvedaClient({ endpoints: { stream: '/sveda/stream' }, fetchFn });
    const session = client.createSession('chat-backend-confirm');
    await session.send('delete it');
    expect(bodies).toHaveLength(1);

    await session.resolveToolConfirmation('tc1', 'approve');

    const secondBody = JSON.parse(bodies[1]);
    expect(secondBody.prompt).toBe('');
    expect(secondBody.toolDecisions).toEqual([{ toolCallId: 'tc1', decision: 'approve' }]);
    const original = session.messages.find(message =>
      message.parts.some(part => part.type === 'tool-call' && part.toolCallId === 'tc1')
    );
    expect(original?.parts.some(part => part.type === 'tool-result')).toBe(true);
    expect(messageText(lastAssistant(session.messages))).toBe('Removed');
    expect(lastAssistant(session.messages).id).not.toBe(original?.id);
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
