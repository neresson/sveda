import { describe, expect, it } from 'vitest';
import { createApp, defineComponent } from 'vue';
import { createVeda, useVedaClient, useVedaConfig } from '../src/plugin';
import { useVedaT } from '../src/i18n/index';

describe('createVeda', () => {
  it('creates client, i18n and config with defaults', () => {
    const veda = createVeda({ endpoints: { stream: '/veda/stream' } });

    expect(veda.client).toBeDefined();
    expect(veda.i18n.t('newChat')).toBe('New Chat');
    expect(veda.config.brand.name).toBe('Veda');
    expect(veda.config.models).toEqual([]);
  });

  it('merges chat component locales', () => {
    const veda = createVeda({ endpoints: { stream: '/veda/stream' } });

    expect(veda.i18n.t('typeMessage')).toBe('Ask AI to help you...');
  });

  it('applies host message overrides last', () => {
    const veda = createVeda({
      endpoints: { stream: '/veda/stream' },
      messages: { en: { newChat: 'Start fresh' } },
    });

    expect(veda.i18n.t('newChat')).toBe('Start fresh');
  });

  it('applies brand, models and quick prompts from options', () => {
    const veda = createVeda({
      endpoints: { stream: '/veda/stream' },
      brand: { name: 'ScorpioGPT' },
      models: [{ id: 'm1', label: 'Model One', supportsThinking: true }],
      quickPrompts: [{ label: 'Help', prompt: 'help me' }],
    });

    expect(veda.config.brand.name).toBe('ScorpioGPT');
    expect(veda.config.models).toHaveLength(1);
    expect(veda.config.quickPrompts).toHaveLength(1);
  });

  it('installs into a Vue app and provides client, config and i18n', () => {
    const veda = createVeda({ endpoints: { stream: '/veda/stream' } });

    let captured: { client: unknown; brand: string; translated: string } | null = null;

    const Probe = defineComponent({
      setup() {
        const client = useVedaClient();
        const config = useVedaConfig();
        const t = useVedaT();
        captured = { client, brand: config.brand.name, translated: t('newChat') };
        return () => null;
      },
    });

    const app = createApp(Probe);
    veda.install(app);
    app.mount(document.createElement('div'));

    expect(captured).not.toBeNull();
    expect(captured!.brand).toBe('Veda');
    expect(captured!.translated).toBe('New Chat');
  });
});
