import { computed, createApp, defineComponent, nextTick } from 'vue';
import { afterEach, describe, expect, it } from 'vitest';
import { applySvedaAppearance } from '../src/appearance';
import { createSveda } from '../src/plugin';
import { useSvedaModel, type SvedaChatModelOption } from '../src/composables/useSvedaModel';

afterEach(() => {
  applySvedaAppearance(null);
});

describe('useSvedaModel', () => {
  const models: SvedaChatModelOption[] = [
    { id: 'flash', label: 'Flash', supportsThinking: true },
    { id: 'plain', label: 'Plain', supportsThinking: false },
  ];

  const mountModel = (localeStorage?: Partial<Record<string, string>>) => {
    window.localStorage.clear();
    for (const [key, value] of Object.entries(localeStorage ?? {})) {
      window.localStorage.setItem(key, value);
    }

    const sveda = createSveda({
      endpoints: { stream: '/sveda/stream' },
      models,
    });

    let captured: ReturnType<typeof useSvedaModel> | null = null;
    const Probe = defineComponent({
      setup() {
        captured = useSvedaModel(computed(() => models));
        return () => null;
      },
    });
    const app = createApp(Probe);
    sveda.install(app);
    app.mount(document.createElement('div'));
    return captured!;
  };

  it('sends thinking true for a model that supports it', () => {
    const model = mountModel();

    expect(model.selectedChatModel.value).toBe('flash');
    expect(model.thinkingEnabled.value).toBe(true);
    expect(model.resolveStreamingSendOptions()).toEqual({
      model: 'flash',
      options: { thinking: true },
    });
  });

  it('sends thinking false when the switch is off or the model cannot think', () => {
    const model = mountModel({ 'sveda.chat-thinking': '0' });
    expect(model.resolveStreamingSendOptions().options).toEqual({ thinking: false });

    model.thinkingEnabled.value = true;
    model.selectedChatModel.value = 'plain';
    expect(model.resolveStreamingSendOptions()).toEqual({
      model: 'plain',
      options: { thinking: false },
    });
  });

  it('keeps the thinking tooltip and persists the switch', async () => {
    const model = mountModel();
    expect(model.thinkingTooltipText.value).toContain('Chain-of-thought reasoning before the answer');
    model.thinkingEnabled.value = false;
    await nextTick();
    expect(window.localStorage.getItem('sveda.chat-thinking')).toBe('0');
  });

  it('forces thinking off when chrome hides the switch', () => {
    window.localStorage.clear();
    const sveda = createSveda({
      endpoints: { stream: '/sveda/stream' },
      models,
      appearance: { chrome: { thinking: false } },
    });
    let captured: ReturnType<typeof useSvedaModel> | null = null;
    const Probe = defineComponent({
      setup() {
        captured = useSvedaModel(computed(() => models));
        return () => null;
      },
    });
    const app = createApp(Probe);
    sveda.install(app);
    app.mount(document.createElement('div'));
    expect(captured!.resolveStreamingSendOptions().options).toEqual({ thinking: false });
  });
});
