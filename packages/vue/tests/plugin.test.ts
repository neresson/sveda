import { describe, expect, it } from 'vitest';
import { createApp, defineComponent, inject } from 'vue';
import { createSveda, useSvedaClient, useSvedaConfig, SvedaFillHostKey } from '../src/plugin';
import { useSvedaT } from '../src/i18n/index';

describe('createSveda', () => {
  it('creates client, i18n and config with defaults', () => {
    const sveda = createSveda({ endpoints: { stream: '/sveda/stream' } });

    expect(sveda.client).toBeDefined();
    expect(sveda.i18n.t('newChat')).toBe('New Chat');
    expect(sveda.config.brand.name).toBe('Sveda');
    expect(sveda.config.models).toEqual([]);
    expect(sveda.client.credentials).toBeUndefined();
  });

  it('forwards fetch credentials to the client', () => {
    const sveda = createSveda({
      endpoints: { stream: '/sveda/stream' },
      credentials: 'include',
    });

    expect(sveda.client.credentials).toBe('include');
  });

  it('merges chat component locales', () => {
    const sveda = createSveda({ endpoints: { stream: '/sveda/stream' } });

    expect(sveda.i18n.t('typeMessage')).toBe('Ask AI to help you...');
  });

  it('applies host message overrides last', () => {
    const sveda = createSveda({
      endpoints: { stream: '/sveda/stream' },
      messages: { en: { newChat: 'Start fresh' } },
    });

    expect(sveda.i18n.t('newChat')).toBe('Start fresh');
  });

  it('applies brand, models and quick prompts from options', () => {
    const sveda = createSveda({
      endpoints: { stream: '/sveda/stream' },
      brand: { name: 'ScorpioGPT' },
      models: [{ id: 'm1', label: 'Model One', supportsThinking: true }],
      quickPrompts: [{ label: 'Help', prompt: 'help me' }],
    });

    expect(sveda.config.brand.name).toBe('ScorpioGPT');
    expect(sveda.config.models).toHaveLength(1);
    expect(sveda.config.quickPrompts).toHaveLength(1);
  });

  it('installs into a Vue app and provides client, config and i18n', () => {
    const sveda = createSveda({ endpoints: { stream: '/sveda/stream' } });

    let captured: { client: unknown; brand: string; translated: string } | null = null;

    const Probe = defineComponent({
      setup() {
        const client = useSvedaClient();
        const config = useSvedaConfig();
        const t = useSvedaT();
        captured = { client, brand: config.brand.name, translated: t('newChat') };
        return () => null;
      },
    });

    const app = createApp(Probe);
    sveda.install(app);
    app.mount(document.createElement('div'));

    expect(captured).not.toBeNull();
    expect(captured!.brand).toBe('Sveda');
    expect(captured!.translated).toBe('New Chat');
  });

  it('does not fill a host window unless the embed element provides it', () => {
    const sveda = createSveda({ endpoints: { stream: '/sveda/stream' } });
    let fillHost = true;

    const Probe = defineComponent({
      setup() {
        fillHost = inject(SvedaFillHostKey, false);
        return () => null;
      },
    });

    const app = createApp(Probe);
    sveda.install(app);
    app.mount(document.createElement('div'));

    expect(fillHost).toBe(false);
  });

  it('fills the iframe host when hostEmbed is set', () => {
    const sveda = createSveda({ endpoints: { stream: '/sveda/stream' }, hostEmbed: true });
    let fillHost = false;

    const Probe = defineComponent({
      setup() {
        fillHost = inject(SvedaFillHostKey, false);
        return () => null;
      },
    });

    const app = createApp(Probe);
    sveda.install(app);
    app.mount(document.createElement('div'));

    expect(fillHost).toBe(true);
  });
});
