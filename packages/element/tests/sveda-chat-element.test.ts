import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { messageText } from '@sveda-ai/core';

const captured = vi.hoisted(() => ({
  props: null as Record<string, unknown> | null,
}));

vi.mock('@sveda-ai/vue', async importOriginal => {
  const original = await importOriginal<typeof import('@sveda-ai/vue')>();
  const { defineComponent, h } = await import('vue');
  return {
    ...original,
    SvedaChat: defineComponent({
      name: 'SvedaChatStub',
      props: [
        'brandName',
        'brandLogo',
        'pageUrl',
        'models',
        'quickPrompts',
        'onNavigate',
        'notify',
      ],
      setup(props) {
        captured.props = props as unknown as Record<string, unknown>;
        return () => h('div', { 'data-sveda-chat-stub': '' });
      },
    }),
  };
});

import { defineSvedaChatElement, SvedaChatElement } from '../src/sveda-chat-element';

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
    headers: { 'Content-Type': 'application/vnd.sveda.stream+json' },
  });
}

function collectEvents(element: HTMLElement, types: string[]) {
  const events: Array<{ type: string; detail: unknown }> = [];
  for (const type of types) {
    element.addEventListener(type, event => {
      events.push({ type, detail: (event as CustomEvent).detail });
    });
  }
  return events;
}

describe('SvedaChatElement', () => {
  const mounted: HTMLElement[] = [];

  beforeEach(() => {
    defineSvedaChatElement();
    localStorage.clear();
    captured.props = null;
  });

  afterEach(() => {
    while (mounted.length > 0) {
      mounted.pop()?.remove();
    }
    vi.unstubAllGlobals();
  });

  function createElement(attributes: Record<string, string> = {}): SvedaChatElement {
    const element = document.createElement('sveda-chat') as SvedaChatElement;
    for (const [name, value] of Object.entries(attributes)) {
      element.setAttribute(name, value);
    }
    document.body.appendChild(element);
    mounted.push(element);
    return element;
  }

  it('defines the custom element', () => {
    expect(customElements.get('sveda-chat')).toBe(SvedaChatElement);
  });

  it('mounts the Vue app into shadow DOM and emits sveda-ready', () => {
    const element = document.createElement('sveda-chat') as SvedaChatElement;
    const ready = vi.fn();
    element.addEventListener('sveda-ready', ready);
    element.setAttribute('stream-endpoint', '/api/stream');
    document.body.appendChild(element);
    mounted.push(element);

    expect(ready).toHaveBeenCalledTimes(1);
    expect(element.shadowRoot?.querySelector('[data-sveda-root]')).not.toBeNull();
    expect(element.shadowRoot?.querySelector('[data-sveda-chat-stub]')).not.toBeNull();
    expect(element.getSession()).toBeNull();
  });

  it('defers mounting until a stream endpoint is configured', () => {
    const element = createElement();

    expect(element.shadowRoot?.querySelector('[data-sveda-chat-stub]')).toBeNull();
    expect(element.getClient()).toBeNull();

    element.setAttribute('stream-endpoint', '/api/late-stream');

    expect(element.getClient()?.endpoints.stream).toBe('/api/late-stream');
    expect(element.shadowRoot?.querySelector('[data-sveda-chat-stub]')).not.toBeNull();
  });

  it('maps attributes to client and component configuration', () => {
    const element = createElement({
      'stream-endpoint': '/api/stream',
      'histories-endpoint': '/api/histories',
      'protocol-mode': 'vercel',
      'brand-name': 'Scorpio',
      'brand-logo-url': '/logo.svg',
      'page-url': '/courses/1',
    });

    const client = element.getClient();
    expect(client?.endpoints.stream).toBe('/api/stream');
    expect(client?.endpoints.histories).toBe('/api/histories');
    expect(client?.protocolMode).toBe('vercel');

    expect(captured.props?.brandName).toBe('Scorpio');
    expect(captured.props?.brandLogo).toBe('/logo.svg');
    expect(captured.props?.pageUrl).toBe('/courses/1');
  });

  it('merges the config property with attributes taking precedence', () => {
    const element = document.createElement('sveda-chat') as SvedaChatElement;
    element.config = {
      endpoints: { stream: '/config/stream', histories: '/config/histories' },
      brand: { name: 'ConfigBrand' },
      models: [{ id: 'm1', label: 'Model One' }],
      quickPrompts: [{ label: 'Help', prompt: 'help me' }],
    };
    element.setAttribute('stream-endpoint', '/attr/stream');
    document.body.appendChild(element);
    mounted.push(element);

    const client = element.getClient();
    expect(client?.endpoints.stream).toBe('/attr/stream');
    expect(client?.endpoints.histories).toBe('/config/histories');
    expect(captured.props?.brandName).toBe('ConfigBrand');
    expect(captured.props?.models).toEqual([{ id: 'm1', label: 'Model One' }]);
    expect(captured.props?.quickPrompts).toEqual([{ label: 'Help', prompt: 'help me' }]);
  });

  it('remounts when a connection attribute changes', () => {
    const element = createElement({ 'stream-endpoint': '/api/stream' });

    expect(element.getClient()?.endpoints.stream).toBe('/api/stream');

    element.setAttribute('stream-endpoint', '/api/stream-v2');

    expect(element.getClient()?.endpoints.stream).toBe('/api/stream-v2');
  });

  it('applies the dark theme class to the shadow container', () => {
    const element = createElement({ 'stream-endpoint': '/api/stream', theme: 'dark' });

    const root = element.shadowRoot?.querySelector('[data-sveda-root]');
    expect(root?.classList.contains('dark')).toBe(true);

    element.setAttribute('theme', 'light');
    expect(root?.classList.contains('dark')).toBe(false);
  });

  it('opens and closes the chat via methods and attributes', async () => {
    const element = createElement({ 'stream-endpoint': '/api/stream' });

    element.open();
    await Promise.resolve();
    expect(localStorage.getItem('sveda.chat-minimized')).toBe('false');

    element.close();
    expect(localStorage.getItem('sveda.chat-minimized')).toBe('true');

    element.setAttribute('open', '');
    await Promise.resolve();
    expect(localStorage.getItem('sveda.chat-minimized')).toBe('false');

    element.setAttribute('minimized', '');
    expect(localStorage.getItem('sveda.chat-minimized')).toBe('true');
  });

  it('honours the open attribute on initial mount', async () => {
    const element = createElement({ 'stream-endpoint': '/api/stream', open: '' });
    await Promise.resolve();

    expect(localStorage.getItem('sveda.chat-minimized')).toBe('false');
  });

  it('sends messages and re-emits session events as CustomEvents', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        sseResponse([
          'data: {"type":"message.start"}',
          'data: {"type":"text.delta","delta":"Hello"}',
          'data: {"type":"text.delta","delta":" world"}',
          'data: {"type":"message.end","finishReason":"stop"}',
        ])
      )
    );

    const element = createElement({ 'stream-endpoint': '/api/stream' });
    const events = collectEvents(element, [
      'sveda-message',
      'sveda-status',
      'sveda-error',
      'sveda-finish',
    ]);

    await element.sendMessage('hi');

    const session = element.getSession();
    expect(session).not.toBeNull();
    expect(session?.messages.some(message => message.role === 'user')).toBe(true);
    const assistant = session?.messages[session.messages.length - 1];
    expect(assistant ? messageText(assistant) : '').toBe('Hello world');

    const statuses = events
      .filter(event => event.type === 'sveda-status')
      .map(event => (event.detail as { status: string }).status);
    expect(statuses).toContain('submitted');
    expect(statuses).toContain('streaming');
    expect(statuses).toContain('idle');
    expect(events.some(event => event.type === 'sveda-message')).toBe(true);
    expect(events.some(event => event.type === 'sveda-finish')).toBe(true);
    expect(events.some(event => event.type === 'sveda-error')).toBe(false);
  });

  it('emits sveda-error when streaming fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('nope', { status: 500 }))
    );

    const element = createElement({ 'stream-endpoint': '/api/stream' });
    const events = collectEvents(element, ['sveda-error', 'sveda-status']);

    await element.sendMessage('hi');

    expect(events.some(event => event.type === 'sveda-error')).toBe(true);
    const statuses = events
      .filter(event => event.type === 'sveda-status')
      .map(event => (event.detail as { status: string }).status);
    expect(statuses).toContain('error');
  });

  it('rejects sendMessage when not mounted', async () => {
    const element = createElement();

    await expect(element.sendMessage('hi')).rejects.toThrow(/not mounted/);
  });

  it('re-emits navigate and notify callbacks as CustomEvents', () => {
    const element = createElement({ 'stream-endpoint': '/api/stream' });
    const events = collectEvents(element, ['sveda-navigate', 'sveda-notify']);

    const onNavigate = captured.props?.onNavigate as (url: string) => void;
    const notify = captured.props?.notify as (kind: string, message: string) => void;
    onNavigate('/courses/9');
    notify('error', 'Something broke');

    expect(events).toContainEqual({ type: 'sveda-navigate', detail: { url: '/courses/9' } });
    expect(events).toContainEqual({
      type: 'sveda-notify',
      detail: { kind: 'error', message: 'Something broke' },
    });
  });

  it('injects styles into the shadow root', () => {
    const element = createElement({ 'stream-endpoint': '/api/stream' });

    element.setStyles('.sveda-test { color: red; }');

    const root = element.shadowRoot!;
    const styleElement = root.querySelector('style[data-sveda-styles]');
    const adopted = root.adoptedStyleSheets?.length ?? 0;

    expect(styleElement !== null || adopted > 0).toBe(true);
    if (styleElement) {
      expect(styleElement.textContent).toContain('.sveda-test');
    }
  });

  it('unmounts the Vue app when disconnected', () => {
    const element = createElement({ 'stream-endpoint': '/api/stream' });
    expect(element.shadowRoot?.querySelector('[data-sveda-chat-stub]')).not.toBeNull();

    element.remove();
    const index = mounted.indexOf(element);
    if (index !== -1) {
      mounted.splice(index, 1);
    }

    expect(element.getClient()).toBeNull();
    expect(element.shadowRoot?.querySelector('[data-sveda-chat-stub]')).toBeNull();
  });
});
