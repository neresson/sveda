import { describe, expect, it } from 'vitest';
import { getSvedaChatStore } from '@sveda-ai/chat';
import { createSveda, SVEDA_SVELTE_VERSION } from '../src/index.js';
import * as pkg from '../src/index.js';

describe('@sveda-ai/svelte exports', () => {
  it('exports SvedaChat and version', () => {
    expect(SVEDA_SVELTE_VERSION).toBe('0.3.1');
    expect(pkg.SvedaChat).toBeTypeOf('function');
    expect(pkg.SvedaProvider).toBeTypeOf('function');
    expect(pkg.createSveda).toBeTypeOf('function');
    expect(pkg.useSvedaChat).toBeTypeOf('function');
    expect(pkg.useSvedaStreaming).toBeTypeOf('function');
  });
});

describe('getSvedaChatStore wiring', () => {
  it('createSveda client can be attached to the shared chat store', () => {
    const sveda = createSveda({
      endpoints: {
        stream: 'http://127.0.0.1:9/sveda/stream',
        message: 'http://127.0.0.1:9/sveda/message',
        histories: 'http://127.0.0.1:9/sveda/chat-histories',
      },
      locale: 'en',
      brand: { name: 'Test' },
    });

    const store = getSvedaChatStore();
    store.setClient(sveda.client);
    store.setTranslate((key) => sveda.i18n.t(key));

    expect(store.getClient()).toBe(sveda.client);
    expect(sveda.i18n.t('typeMessage')).toBe('Ask AI to help you...');
    expect(sveda.config.brand.name).toBe('Test');

    const chatId = store.createNewChat((key) => sveda.i18n.t(key));
    expect(chatId).toMatch(/^chat_/);
    expect(store.getState().chatHistories.some((h) => h.id === chatId)).toBe(true);
  });
});
