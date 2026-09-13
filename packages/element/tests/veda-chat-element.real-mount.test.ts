import { afterEach, describe, expect, it } from 'vitest';
import { defineVedaChatElement, VedaChatElement } from '../src/veda-chat-element';

describe('VedaChatElement real VedaChat mount', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('mounts the real VedaChat component into shadow DOM', async () => {
    defineVedaChatElement();
    const element = document.createElement('veda-chat') as VedaChatElement;
    element.setAttribute('stream-endpoint', '/api/stream');
    document.body.appendChild(element);

    await new Promise(resolve => setTimeout(resolve, 50));

    const root = element.shadowRoot?.querySelector('[data-veda-root]');
    expect(root).not.toBeNull();
    expect(root?.childElementCount ?? 0).toBeGreaterThan(0);
    expect(element.getClient()?.endpoints.stream).toBe('/api/stream');

    element.remove();
  });
});
