import { afterEach, describe, expect, it } from 'vitest';
import { defineSvedaChatElement, SvedaChatElement } from '../src/sveda-chat-element';

describe('SvedaChatElement real SvedaChat mount', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('mounts the real SvedaChat component into shadow DOM', async () => {
    defineSvedaChatElement();
    const element = document.createElement('sveda-chat') as SvedaChatElement;
    element.setAttribute('stream-endpoint', '/api/stream');
    document.body.appendChild(element);

    await new Promise(resolve => setTimeout(resolve, 50));

    const root = element.shadowRoot?.querySelector('[data-sveda-root]');
    expect(root).not.toBeNull();
    expect(root?.childElementCount ?? 0).toBeGreaterThan(0);
    expect(element.getClient()?.endpoints.stream).toBe('/api/stream');

    element.remove();
  });
});
