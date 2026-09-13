import { computed, type Ref } from 'vue';

export const VEDA_CHAT_MAX_CONTEXT_TOKENS = 200000;

export function useVedaContextWindow(
  currentChat: Ref<{ contextWindowTokens?: number } | null | undefined>,
  maxTokens?: Ref<number>
) {
  const contextTokensUsed = computed(() => {
    return Math.max(0, Math.round(Number(currentChat.value?.contextWindowTokens || 0)));
  });

  const contextWindowMaxTokens = computed(() => {
    const value = Number(maxTokens?.value ?? VEDA_CHAT_MAX_CONTEXT_TOKENS);
    return Number.isFinite(value) && value > 0 ? value : VEDA_CHAT_MAX_CONTEXT_TOKENS;
  });

  const contextWindowUsagePercent = computed(() => {
    if (contextWindowMaxTokens.value <= 0) {
      return 0;
    }
    const raw = (contextTokensUsed.value / contextWindowMaxTokens.value) * 100;
    return Math.max(0, Math.min(100, Math.round(raw)));
  });

  return {
    contextTokensUsed,
    contextWindowMaxTokens,
    contextWindowUsagePercent,
  };
}
