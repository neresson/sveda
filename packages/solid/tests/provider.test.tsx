import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from '@solidjs/testing-library';
import { createSveda, SvedaProvider, useSvedaClient, useSvedaConfig } from '../src/provider';
import { useSvedaT } from '../src/i18n/index';
import { SvedaChat } from '../src/components/SvedaChat';
import { SvedaMinimizedTrigger } from '../src/components/SvedaMinimizedTrigger';

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

  it('fills the iframe host when hostEmbed is set', () => {
    const sveda = createSveda({ endpoints: { stream: '/sveda/stream' }, hostEmbed: true });
    expect(sveda.fillHost).toBe(true);
  });
});

describe('SvedaProvider', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({
        ok: false,
        json: async () => ({}),
      })),
    );
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it('provides client and config to children', () => {
    let brand = '';
    let translated = '';
    let hasClient = false;

    const Probe = () => {
      const client = useSvedaClient();
      const config = useSvedaConfig();
      const t = useSvedaT();
      hasClient = Boolean(client);
      brand = config.brand.name;
      translated = t('newChat');
      return <div data-testid="probe" />;
    };

    render(() => (
      <SvedaProvider endpoints={{ stream: 'http://127.0.0.1:9/sveda/stream' }} locale="en">
        <Probe />
      </SvedaProvider>
    ));

    expect(hasClient).toBe(true);
    expect(brand).toBe('Sveda');
    expect(translated).toBe('New Chat');
  });

  it('renders launcher when chat is minimized', () => {
    const { container } = render(() => (
      <SvedaMinimizedTrigger label="Sveda" icon="sparkles" onOpen={() => {}} />
    ));

    expect(container.textContent).toContain('Sveda');
    expect(container.querySelector('button')).toBeTruthy();
  });

  it('renders SvedaChat shell with mocked network', async () => {
    const { container } = render(() => (
      <SvedaProvider
        endpoints={{
          stream: 'http://127.0.0.1:9/sveda/stream',
          histories: 'http://127.0.0.1:9/sveda/chat-histories',
        }}
        credentials="omit"
        locale="en"
        brand={{ name: 'Solid Test' }}
        hideLauncher={false}
      >
        <SvedaChat brandName="Solid Test" />
      </SvedaProvider>
    ));

    expect(container.querySelector('.sveda-chat')).toBeTruthy();
  });
});
