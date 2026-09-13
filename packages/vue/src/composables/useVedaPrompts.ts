import { computed, type ComputedRef, type Ref } from 'vue';
import { userMessageHasVisibleContent, type VedaUserMessageLike } from '../lib/userMessage';

export interface VedaQuickPromptItem {
  label: string;
  prompt: string;
}

type ChatWithMessages = {
  id?: string;
  messages?: VedaUserMessageLike[];
};

export function useVedaPrompts(
  currentChat: Ref<ChatWithMessages | null | undefined>,
  quickPrompts: ComputedRef<Array<VedaQuickPromptItem | string> | undefined>
) {
  const messages = computed(() => currentChat.value?.messages || []);

  const hasUserMessages = computed(() => messages.value.some(userMessageHasVisibleContent));

  const quickPromptButtons = computed<VedaQuickPromptItem[]>(() => {
    const list = quickPrompts.value;
    if (!Array.isArray(list)) {
      return [];
    }

    return list
      .map(item => {
        if (typeof item === 'string') {
          const label = item.trim();
          return label ? { label, prompt: label } : null;
        }
        const label = String(item?.label ?? '').trim();
        const prompt = String(item?.prompt ?? '').trim();
        return label && prompt ? { label, prompt } : null;
      })
      .filter((item): item is VedaQuickPromptItem => !!item);
  });

  const showQuickPromptButtons = computed(() => {
    if (!currentChat.value) {
      return false;
    }

    if (quickPromptButtons.value.length === 0) {
      return false;
    }

    return !currentChat.value.messages?.some(userMessageHasVisibleContent);
  });

  return {
    messages,
    hasUserMessages,
    quickPromptButtons,
    showQuickPromptButtons,
  };
}
