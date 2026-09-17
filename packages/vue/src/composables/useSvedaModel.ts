import type { SvedaSendOptions } from '@sveda-ai/core';
import { computed, ref, watch, type ComputedRef } from 'vue';
import { useSvedaT } from '../i18n/index';

const SVEDA_CHAT_MODEL_STORAGE_KEY = 'sveda.chat-model';
const SVEDA_CHAT_THINKING_STORAGE_KEY = 'sveda.chat-thinking';

export interface SvedaChatModelOption {
  id: string;
  label: string;
  supportsThinking?: boolean;
}

export function useSvedaModel(models: ComputedRef<SvedaChatModelOption[]>) {
  const t = useSvedaT();

  const readStoredModel = (): string => {
    if (typeof window === 'undefined') {
      return models.value[0]?.id ?? '';
    }
    const savedModel = localStorage.getItem(SVEDA_CHAT_MODEL_STORAGE_KEY);
    if (savedModel && models.value.some(option => option.id === savedModel)) {
      return savedModel;
    }
    return models.value[0]?.id ?? '';
  };

  const readStoredThinking = (): boolean => {
    if (typeof window === 'undefined') {
      return true;
    }
    const savedThinking = localStorage.getItem(SVEDA_CHAT_THINKING_STORAGE_KEY);
    if (savedThinking === '0') {
      return false;
    }
    return true;
  };

  const selectedChatModel = ref(readStoredModel());
  const thinkingEnabled = ref(readStoredThinking());

  const chatModels = computed(() => models.value);

  const selectedModelSupportsThinking = computed(
    () =>
      models.value.find(option => option.id === selectedChatModel.value)?.supportsThinking ??
      false
  );

  const thinkingTooltipText = computed(() => t('thinkingTooltip'));

  watch(models, list => {
    if (list.length > 0 && !list.some(option => option.id === selectedChatModel.value)) {
      selectedChatModel.value = list[0].id;
    }
  });

  watch(selectedChatModel, newVal => {
    if (typeof window !== 'undefined' && newVal) {
      localStorage.setItem(SVEDA_CHAT_MODEL_STORAGE_KEY, newVal);
    }
  });

  watch(thinkingEnabled, newVal => {
    if (typeof window !== 'undefined') {
      localStorage.setItem(SVEDA_CHAT_THINKING_STORAGE_KEY, newVal ? '1' : '0');
    }
  });

  const resolveStreamingSendOptions = (): Pick<SvedaSendOptions, 'model' | 'options'> => ({
    model: selectedChatModel.value || undefined,
    options: {
      thinking: Boolean(selectedModelSupportsThinking.value && thinkingEnabled.value),
    },
  });

  return {
    selectedChatModel,
    thinkingEnabled,
    chatModels,
    selectedModelSupportsThinking,
    thinkingTooltipText,
    resolveStreamingSendOptions,
  };
}
