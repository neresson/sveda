import { describe, expect, it } from 'vitest';
import {
  createSvedaEmbedProtocol,
  SVEDA_EMBED_SOURCE,
  SVEDA_EMBED_VERSION,
} from '../src/embed-protocol';

describe('createSvedaEmbedProtocol', () => {
  it('round-trips an encoded envelope', () => {
    const protocol = createSvedaEmbedProtocol();
    const envelope = protocol.encode('setContext', { context: { page: 'dashboard' } });

    expect(envelope).toEqual({
      source: SVEDA_EMBED_SOURCE,
      version: SVEDA_EMBED_VERSION,
      type: 'setContext',
      payload: { context: { page: 'dashboard' } },
    });

    const decoded = protocol.decode(JSON.parse(JSON.stringify(envelope)));
    expect(decoded).toEqual(envelope);
  });

  it('rejects foreign or malformed messages', () => {
    const protocol = createSvedaEmbedProtocol();

    expect(protocol.decode(null)).toBeNull();
    expect(protocol.decode(undefined)).toBeNull();
    expect(protocol.decode('sveda-embed')).toBeNull();
    expect(protocol.decode({ type: 'ready' })).toBeNull();
    expect(protocol.decode({ source: 'other', version: 1, type: 'ready' })).toBeNull();
    expect(protocol.decode({ source: SVEDA_EMBED_SOURCE, version: 2, type: 'ready' })).toBeNull();
    expect(protocol.decode({ source: SVEDA_EMBED_SOURCE, version: 1 })).toBeNull();
  });

  it('classifies host commands and embed events', () => {
    const protocol = createSvedaEmbedProtocol();

    expect(protocol.isHostCommand(protocol.encode('ack', { version: 1 }))).toBe(true);
    expect(protocol.isHostCommand(protocol.encode('setTheme', { theme: 'dark' }))).toBe(true);
    expect(protocol.isHostCommand(protocol.encode('ready', { version: 1 }))).toBe(false);

    expect(protocol.isEmbedEvent(protocol.encode('ready', { version: 1 }))).toBe(true);
    expect(protocol.isEmbedEvent(protocol.encode('navigate', { url: '/x' }))).toBe(true);
    expect(protocol.isEmbedEvent(protocol.encode('open', {}))).toBe(false);
  });
});
