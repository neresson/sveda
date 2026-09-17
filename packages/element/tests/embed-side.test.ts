import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSvedaEmbedProtocol } from '../src/embed-protocol';
import { initSvedaEmbedHost, type SvedaEmbedHost } from '../src/embed-side';

const HOST_ORIGIN = 'https://host.example.com';

function createFakeTarget() {
  const posted: Array<{ data: unknown; origin: string }> = [];
  const target = {
    postMessage: vi.fn((data: unknown, origin: string) => {
      posted.push({ data, origin });
    }),
  } as unknown as Window;
  return { target, posted };
}

function dispatchToEmbed(data: unknown, origin: string, source: unknown): void {
  const event = new MessageEvent('message', { data, origin });
  Object.defineProperty(event, 'source', { value: source });
  window.dispatchEvent(event);
}

describe('initSvedaEmbedHost', () => {
  const hosts: SvedaEmbedHost[] = [];

  afterEach(() => {
    while (hosts.length > 0) {
      hosts.pop()?.disconnect();
    }
    vi.useRealTimers();
  });

  const init = (
    handlers: Parameters<typeof initSvedaEmbedHost>[0] = {},
    options: Parameters<typeof initSvedaEmbedHost>[1] = {}
  ) => {
    const fake = createFakeTarget();
    const host = initSvedaEmbedHost(handlers, {
      target: fake.target,
      targetOrigin: HOST_ORIGIN,
      readyRetryInterval: 0,
      ...options,
    });
    hosts.push(host);
    return { ...fake, host };
  };

  it('posts ready on init', () => {
    const { posted } = init();

    expect(posted).toHaveLength(1);
    expect(posted[0].data).toMatchObject({
      source: 'sveda-embed',
      version: 1,
      type: 'ready',
    });
    expect(posted[0].origin).toBe(HOST_ORIGIN);
  });

  it('marks acknowledged on ack and stops retrying', () => {
    vi.useFakeTimers();
    const onAck = vi.fn();
    const { host, target, posted } = init(
      { onAck },
      { readyRetryInterval: 100, maxReadyAttempts: 10 }
    );
    const protocol = createSvedaEmbedProtocol();

    expect(host.acknowledged).toBe(false);
    vi.advanceTimersByTime(250);
    const readyCount = posted.filter(
      entry => (entry.data as { type: string }).type === 'ready'
    ).length;
    expect(readyCount).toBeGreaterThan(1);

    dispatchToEmbed(protocol.encode('ack', { version: 1 }), HOST_ORIGIN, target);

    expect(host.acknowledged).toBe(true);
    expect(onAck).toHaveBeenCalledTimes(1);

    const countAfterAck = posted.length;
    vi.advanceTimersByTime(1000);
    expect(posted).toHaveLength(countAfterAck);
  });

  it('dispatches host commands to handlers', () => {
    const handlers = {
      onSetContext: vi.fn(),
      onSetAuthToken: vi.fn(),
      onOpen: vi.fn(),
      onClose: vi.fn(),
      onSendMessage: vi.fn(),
      onSetLocale: vi.fn(),
      onSetTheme: vi.fn(),
    };
    const { target } = init(handlers);
    const protocol = createSvedaEmbedProtocol();

    dispatchToEmbed(protocol.encode('setContext', { context: { page: 'dashboard' } }), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('setAuthToken', { token: 'abc' }), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('open', {}), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('close', {}), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('sendMessage', { text: 'hi', context: { a: 1 } }), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('setLocale', { locale: 'ru' }), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('setTheme', { theme: 'dark' }), HOST_ORIGIN, target);

    expect(handlers.onSetContext).toHaveBeenCalledWith({ page: 'dashboard' });
    expect(handlers.onSetAuthToken).toHaveBeenCalledWith('abc');
    expect(handlers.onOpen).toHaveBeenCalledTimes(1);
    expect(handlers.onClose).toHaveBeenCalledTimes(1);
    expect(handlers.onSendMessage).toHaveBeenCalledWith('hi', { a: 1 });
    expect(handlers.onSetLocale).toHaveBeenCalledWith('ru');
    expect(handlers.onSetTheme).toHaveBeenCalledWith('dark');
  });

  it('rejects commands from disallowed origins when configured', () => {
    const onOpen = vi.fn();
    const { target } = init({ onOpen }, { allowedOrigins: [HOST_ORIGIN] });
    const protocol = createSvedaEmbedProtocol();

    dispatchToEmbed(protocol.encode('open', {}), 'https://evil.example.com', target);
    expect(onOpen).not.toHaveBeenCalled();

    dispatchToEmbed(protocol.encode('open', {}), HOST_ORIGIN, target);
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it('ignores foreign envelopes and invalid payloads', () => {
    const onSendMessage = vi.fn();
    const onSetTheme = vi.fn();
    const { target } = init({ onSendMessage, onSetTheme });
    const protocol = createSvedaEmbedProtocol();

    dispatchToEmbed({ type: 'sendMessage', payload: { text: 'hi' } }, HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('sendMessage', { text: '' }), HOST_ORIGIN, target);
    dispatchToEmbed(protocol.encode('setTheme', { theme: 'blue' }), HOST_ORIGIN, target);

    expect(onSendMessage).not.toHaveBeenCalled();
    expect(onSetTheme).not.toHaveBeenCalled();
  });

  it('posts resize, navigate, error and tool progress events', () => {
    const { host, posted } = init();
    posted.length = 0;

    host.postResize(512);
    host.postNavigate('/courses/2');
    host.postError('failed');
    host.postToolProgress({ toolName: 'search' });

    expect(posted.map(entry => (entry.data as { type: string }).type)).toEqual([
      'resize',
      'navigate',
      'error',
      'toolProgress',
    ]);
    expect(posted[0].data).toMatchObject({ payload: { height: 512 } });
    expect(posted[1].data).toMatchObject({ payload: { url: '/courses/2' } });
    expect(posted[2].data).toMatchObject({ payload: { message: 'failed' } });
    expect(posted[3].data).toMatchObject({ payload: { event: { toolName: 'search' } } });
  });
});
