import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSvedaEmbedProtocol } from '../src/embed-protocol';
import { connectSvedaEmbed, type SvedaEmbedConnection } from '../src/host-sdk';

const EMBED_ORIGIN = 'https://embed.example.com';

function createFakeIframe(src = `${EMBED_ORIGIN}/chat`) {
  const posted: Array<{ data: unknown; origin: string }> = [];
  const contentWindow = {
    postMessage: vi.fn((data: unknown, origin: string) => {
      posted.push({ data, origin });
    }),
  };
  const iframe = { src, contentWindow } as unknown as HTMLIFrameElement;
  return { iframe, contentWindow, posted };
}

function dispatchToHost(data: unknown, origin: string, source: unknown): void {
  const event = new MessageEvent('message', { data, origin });
  Object.defineProperty(event, 'source', { value: source });
  window.dispatchEvent(event);
}

describe('connectSvedaEmbed', () => {
  const connections: SvedaEmbedConnection[] = [];

  afterEach(() => {
    while (connections.length > 0) {
      connections.pop()?.disconnect();
    }
  });

  const connect = (
    options: Partial<Parameters<typeof connectSvedaEmbed>[1]> = {},
    iframeSrc?: string
  ) => {
    const fake = createFakeIframe(iframeSrc);
    const connection = connectSvedaEmbed(fake.iframe, {
      allowedOrigins: [EMBED_ORIGIN],
      ...options,
    });
    connections.push(connection);
    return { ...fake, connection };
  };

  it('completes the ready/ack handshake', () => {
    const onReady = vi.fn();
    const { connection, contentWindow, posted } = connect({ onReady });
    const protocol = createSvedaEmbedProtocol();

    expect(connection.ready).toBe(false);

    dispatchToHost(protocol.encode('ready', { version: 1 }), EMBED_ORIGIN, contentWindow);

    expect(connection.ready).toBe(true);
    expect(onReady).toHaveBeenCalledTimes(1);
    expect(posted).toHaveLength(1);
    expect(posted[0].data).toMatchObject({ type: 'ack', source: 'sveda-embed', version: 1 });
    expect(posted[0].origin).toBe(EMBED_ORIGIN);
  });

  it('rejects messages from disallowed origins', () => {
    const onReady = vi.fn();
    const onNavigate = vi.fn();
    const { contentWindow } = connect({ onReady, onNavigate });
    const protocol = createSvedaEmbedProtocol();

    dispatchToHost(protocol.encode('ready', { version: 1 }), 'https://evil.example.com', contentWindow);
    dispatchToHost(protocol.encode('navigate', { url: 'https://evil.example.com' }), 'https://evil.example.com', contentWindow);

    expect(onReady).not.toHaveBeenCalled();
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('ignores messages from other sources and foreign envelopes', () => {
    const onReady = vi.fn();
    const { contentWindow } = connect({ onReady });
    const protocol = createSvedaEmbedProtocol();

    dispatchToHost(protocol.encode('ready', { version: 1 }), EMBED_ORIGIN, {});
    dispatchToHost({ type: 'ready' }, EMBED_ORIGIN, contentWindow);
    dispatchToHost(null, EMBED_ORIGIN, contentWindow);

    expect(onReady).not.toHaveBeenCalled();
  });

  it('queues host commands until the embed is ready, then flushes in order', () => {
    const { connection, contentWindow, posted } = connect();
    const protocol = createSvedaEmbedProtocol();

    connection.setContext({ page: 'dashboard' });
    connection.open();

    expect(posted).toHaveLength(0);

    dispatchToHost(protocol.encode('ready', { version: 1 }), EMBED_ORIGIN, contentWindow);

    expect(posted).toHaveLength(3);
    expect(posted[0].data).toMatchObject({ type: 'ack' });
    expect(posted[1].data).toMatchObject({
      type: 'setContext',
      payload: { context: { page: 'dashboard' } },
    });
    expect(posted[2].data).toMatchObject({ type: 'open' });
  });

  it('sends all host commands once ready', () => {
    const { connection, contentWindow, posted } = connect();
    const protocol = createSvedaEmbedProtocol();
    dispatchToHost(protocol.encode('ready', { version: 1 }), EMBED_ORIGIN, contentWindow);
    posted.length = 0;

    connection.setAuthToken('token-123');
    connection.close();
    connection.sendMessage('hello', { page: 'x' });
    connection.setLocale('ru');
    connection.setTheme('dark');

    expect(posted.map(entry => (entry.data as { type: string }).type)).toEqual([
      'setAuthToken',
      'close',
      'sendMessage',
      'setLocale',
      'setTheme',
    ]);
    expect(posted[0].data).toMatchObject({ payload: { token: 'token-123' } });
    expect(posted[2].data).toMatchObject({ payload: { text: 'hello', context: { page: 'x' } } });
    expect(posted[3].data).toMatchObject({ payload: { locale: 'ru' } });
    expect(posted[4].data).toMatchObject({ payload: { theme: 'dark' } });
    expect(posted.every(entry => entry.origin === EMBED_ORIGIN)).toBe(true);
  });

  it('routes embed events to handlers', () => {
    const onResize = vi.fn();
    const onNavigate = vi.fn();
    const onError = vi.fn();
    const onToolProgress = vi.fn();
    const { contentWindow } = connect({ onResize, onNavigate, onError, onToolProgress });
    const protocol = createSvedaEmbedProtocol();

    dispatchToHost(protocol.encode('resize', { height: 480 }), EMBED_ORIGIN, contentWindow);
    dispatchToHost(protocol.encode('navigate', { url: '/courses/1' }), EMBED_ORIGIN, contentWindow);
    dispatchToHost(protocol.encode('error', { message: 'boom' }), EMBED_ORIGIN, contentWindow);
    dispatchToHost(
      protocol.encode('toolProgress', { event: { toolName: 'search' } }),
      EMBED_ORIGIN,
      contentWindow
    );

    expect(onResize).toHaveBeenCalledWith(480);
    expect(onNavigate).toHaveBeenCalledWith('/courses/1');
    expect(onError).toHaveBeenCalledWith({ message: 'boom' });
    expect(onToolProgress).toHaveBeenCalledWith({ toolName: 'search' });
  });

  it('stops listening after disconnect', () => {
    const onReady = vi.fn();
    const { connection, contentWindow, posted } = connect({ onReady });
    const protocol = createSvedaEmbedProtocol();

    connection.disconnect();
    dispatchToHost(protocol.encode('ready', { version: 1 }), EMBED_ORIGIN, contentWindow);

    expect(onReady).not.toHaveBeenCalled();
    expect(connection.ready).toBe(false);
    expect(posted).toHaveLength(0);
  });
});
