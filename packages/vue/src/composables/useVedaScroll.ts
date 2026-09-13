import { nextTick, ref, watch, type ComputedRef, type Ref } from 'vue';

export type VedaScrollContainer = {
  scrollToBottom?: (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => void;
} | null;

export function useVedaScroll(
  messagesContainerRef: Ref<VedaScrollContainer>,
  currentChatId: ComputedRef<string | undefined>,
  pageUrl: ComputedRef<string>,
  messages: ComputedRef<unknown[]>,
  isStreaming: Ref<boolean>
) {
  const isMessagesNearBottom = ref(true);
  const showAgentCompletedNotice = ref(false);
  const streamingScrollGuardEnabled = ref(false);

  const scrollToBottom = (options: { behavior?: string; onlyIfNearBottom?: boolean } = {}) => {
    nextTick(() => {
      if (messagesContainerRef.value?.scrollToBottom) {
        messagesContainerRef.value.scrollToBottom({
          ...options,
          onlyIfNearBottom:
            options.onlyIfNearBottom ??
            (streamingScrollGuardEnabled.value || showAgentCompletedNotice.value),
        });
      }
    });
  };

  const handleMessagesNearBottomChange = (value: boolean) => {
    isMessagesNearBottom.value = value;
    if (value) {
      showAgentCompletedNotice.value = false;
    }
  };

  const scrollToCompletedAnswer = () => {
    showAgentCompletedNotice.value = false;
    scrollToBottom({ behavior: 'smooth', onlyIfNearBottom: false });
  };

  const resetAgentCompletedNotice = () => {
    showAgentCompletedNotice.value = false;
  };

  watch(
    isStreaming,
    (value, oldValue) => {
      streamingScrollGuardEnabled.value = value;
      if (value) {
        showAgentCompletedNotice.value = false;
        return;
      }

      if (oldValue && !isMessagesNearBottom.value) {
        showAgentCompletedNotice.value = true;
      }
    },
    { flush: 'sync' }
  );

  watch(
    () => currentChatId.value,
    () => {
      scrollToBottom();
    }
  );

  watch(
    () => pageUrl.value,
    () => {
      scrollToBottom();
    }
  );

  watch(
    () => messages.value,
    () => {
      scrollToBottom();
    },
    { deep: true }
  );

  return {
    isMessagesNearBottom,
    showAgentCompletedNotice,
    scrollToBottom,
    handleMessagesNearBottomChange,
    scrollToCompletedAnswer,
    resetAgentCompletedNotice,
  };
}
