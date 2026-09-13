import { describe, expect, it } from 'vitest';
import {
  createVedaEmbedProtocol,
  VEDA_EMBED_SOURCE,
  VEDA_EMBED_VERSION,
} from '../src/embed-protocol';

describe('createVedaEmbedProtocol', () => {
  it('round-trips an encoded envelope', () => {
    const protocol = createVedaEmbedProtocol();
    const envelope = protocol.encode('setContext', { context: { page: 'dashboard' } });

    expect(envelope).toEqual({
      source: VEDA_EMBED_SOURCE,
      version: VEDA_EMBED_VERSION,
      type: 'setContext',
      payload: { context: { page: 'dashboard' } },
    });

    const decoded = protocol.decode(JSON.parse(JSON.stringify(envelope)));
    expect(decoded).toEqual(envelope);
  });

  it('rejects foreign or malformed messages', () => {
    const protocol = createVedaEmbedProtocol();

    expect(protocol.decode(null)).toBeNull();
    expect(protocol.decode(undefined)).toBeNull();
    expect(protocol.decode('veda-embed')).toBeNull();
    expect(protocol.decode({ type: 'ready' })).toBeNull();
    expect(protocol.decode({ source: 'other', version: 1, type: 'ready' })).toBeNull();
    expect(protocol.decode({ source: VEDA_EMBED_SOURCE, version: 2, type: 'ready' })).toBeNull();
    expect(protocol.decode({ source: VEDA_EMBED_SOURCE, version: 1 })).toBeNull();
  });

  it('classifies host commands and embed events', () => {
    const protocol = createVedaEmbedProtocol();

    expect(protocol.isHostCommand(protocol.encode('ack', { version: 1 }))).toBe(true);
    expect(protocol.isHostCommand(protocol.encode('setTheme', { theme: 'dark' }))).toBe(true);
    expect(protocol.isHostCommand(protocol.encode('ready', { version: 1 }))).toBe(false);

    expect(protocol.isEmbedEvent(protocol.encode('ready', { version: 1 }))).toBe(true);
    expect(protocol.isEmbedEvent(protocol.encode('navigate', { url: '/x' }))).toBe(true);
    expect(protocol.isEmbedEvent(protocol.encode('open', {}))).toBe(false);
  });
});
