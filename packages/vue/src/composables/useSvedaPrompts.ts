import { computed, type ComputedRef, type Ref } from 'vue';
import { userMessageHasVisibleContent, type SvedaUserMessageLike } from '../lib/userMessage';

export interface SvedaQuickPromptItem {
  label: string;
  prompt: string;
}

type ChatWithMessages = {
  id?: string;
  messages?: SvedaUserMessageLike[];
};

export function useSvedaPrompts(
  currentChat: Ref<ChatWithMessages | null | undefined>,
  quickPrompts: ComputedRef<Array<SvedaQuickPromptItem | string> | undefined>
) {
  const messages = computed(() => currentChat.value?.messages || []);

  const hasUserMessages = computed(() => messages.value.some(userMessageHasVisibleContent));

  const quickPromptButtons = computed<SvedaQuickPromptItem[]>(() => {
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
      .filter((item): item is SvedaQuickPromptItem => !!item);
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
