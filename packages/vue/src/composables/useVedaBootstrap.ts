import { nextTick, type Ref } from 'vue';

const PAGE_SESSION_BOOT_KEY = '__VEDA_CHAT_PAGE_BOOTED__';

type SortedHistory = { id: string };

export function useVedaBootstrap(deps: {
  loadHistories: () => Promise<void>;
  createNewChat: () => void;
  setCurrentChat: (id: string) => Promise<void>;
  currentChat: Ref<{ id?: string } | null | undefined>;
  sortedHistories: Ref<SortedHistory[]>;
  scrollToBottom: () => void;
}) {
  const loadChatBootstrap = async () => {
    await deps.loadHistories();
    const globalScope = window as unknown as Record<string, unknown>;
    const isPageBooted = Boolean(globalScope[PAGE_SESSION_BOOT_KEY]);
    if (!isPageBooted) {
      globalScope[PAGE_SESSION_BOOT_KEY] = true;
      deps.createNewChat();
    } else if (!deps.currentChat.value) {
      if (deps.sortedHistories.value.length > 0) {
        await deps.setCurrentChat(deps.sortedHistories.value[0].id);
      } else {
        deps.createNewChat();
      }
    }
    await nextTick();
    deps.scrollToBottom();
  };

  return {
    loadChatBootstrap,
  };
}
