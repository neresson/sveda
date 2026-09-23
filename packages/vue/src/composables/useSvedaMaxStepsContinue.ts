import { computed, ref, type ComputedRef, type Ref } from 'vue';

export type SvedaMaxStepsReachedData = {
  maxSteps: number;
};

type PendingMaxSteps = Record<string, SvedaMaxStepsReachedData>;

export function useSvedaMaxStepsContinue(
  currentChatId: ComputedRef<string | null | undefined>,
  isStreaming: Ref<boolean>,
  isLoading: Ref<boolean>
) {
  const pendingByChatId = ref<PendingMaxSteps>({});

  const showMaxStepsContinueNotice = computed(() => {
    const chatId = currentChatId.value;
    if (!chatId) {
      return false;
    }

    return chatId in pendingByChatId.value;
  });

  const maxStepsContinueLimit = computed(() => {
    const chatId = currentChatId.value;
    if (!chatId) {
      return 0;
    }

    return pendingByChatId.value[chatId]?.maxSteps ?? 0;
  });

  const markMaxStepsReached = (chatId: string, data: SvedaMaxStepsReachedData) => {
    pendingByChatId.value = {
      ...pendingByChatId.value,
      [chatId]: { maxSteps: data.maxSteps },
    };
  };

  const resetMaxStepsContinueNotice = (chatId?: string) => {
    if (!chatId) {
      pendingByChatId.value = {};

      return;
    }

    if (!(chatId in pendingByChatId.value)) {
      return;
    }

    const next = { ...pendingByChatId.value };
    delete next[chatId];
    pendingByChatId.value = next;
  };

  const maxStepsContinueDisabled = computed(
    () => isStreaming.value || isLoading.value || !showMaxStepsContinueNotice.value
  );

  return {
    showMaxStepsContinueNotice,
    maxStepsContinueLimit,
    maxStepsContinueDisabled,
    markMaxStepsReached,
    resetMaxStepsContinueNotice,
  };
}
