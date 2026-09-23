import {
  getSvedaChatStore,
  type SvedaChatHistory,
  type SvedaChatState,
} from '@sveda-ai/chat';
import { SVEDA_EMBED_AUTH_EVENT, type SvedaContext } from '../provider.js';
import { useSvedaContext, useSvedaT } from '../context.js';

export type { SvedaChatHistory, SvedaChatState };

let initialized = false;
let subscribed = false;

const store = getSvedaChatStore({ embedAuthEvent: SVEDA_EMBED_AUTH_EVENT });

export function useSvedaChat(sveda?: SvedaContext) {
  let ctx: SvedaContext | null = sveda ?? null;
  try {
    ctx = sveda ?? useSvedaContext();
  } catch {
    ctx = sveda ?? null;
  }

  const t = (() => {
    try {
      return useSvedaT();
    } catch {
      return (key: string) => key;
    }
  })();

  if (ctx?.client) {
    store.setClient(ctx.client);
  }
  store.setTranslate(t);

  const initial = store.getState();

  let chatState = $state<SvedaChatState>({ ...initial.chatState });
  let currentChat = $state<SvedaChatHistory | null>(initial.currentChat);
  let chatHistories = $state<SvedaChatHistory[]>(initial.chatHistories);
  let storeRevision = $state(0);

  const syncFromStore = () => {
    const snapshot = store.getState();
    chatState = { ...snapshot.chatState };
    currentChat = snapshot.currentChat;
    chatHistories = snapshot.chatHistories;
    storeRevision += 1;
  };

  if (!subscribed) {
    subscribed = true;
    store.subscribe(syncFromStore);
  }

  if (!initialized) {
    initialized = true;
    store.initAuthListener();
    void store.loadHistories();
  }

  return {
    get chatState() {
      return chatState;
    },
    get currentChat() {
      return currentChat;
    },
    get chatHistories() {
      return chatHistories;
    },
    get sortedHistories() {
      void storeRevision;
      return store.sortedHistories();
    },
    get openChatTabs() {
      void storeRevision;
      return store.openChatTabs();
    },
    get isOpen() {
      return chatState.isOpen;
    },
    get isMinimized() {
      return chatState.isMinimized;
    },
    get isLoading() {
      return chatState.isLoading;
    },
    get isLoadingChatHistory() {
      return chatState.isLoadingChatHistory;
    },
    get hasCurrentChat() {
      return !!currentChat;
    },
    loadHistories: () => store.loadHistories(),
    createNewChat: () => store.createNewChat(t),
    setCurrentChat: (chatId: string) => store.setCurrentChat(chatId, t),
    addMessage: store.addMessage.bind(store),
    setChatTitle: store.setChatTitle.bind(store),
    refreshChatTitleFromServer: store.refreshChatTitleFromServer.bind(store),
    updateMessage: store.updateMessage.bind(store),
    setChatMessages: store.setChatMessages.bind(store),
    incrementChatTokens: store.incrementChatTokens.bind(store),
    setChatContextWindowTokens: store.setChatContextWindowTokens.bind(store),
    deleteChat: (chatId: string) => store.deleteChat(chatId, t),
    renameChat: (chatId: string, title: string) => store.renameChat(chatId, title),
    closeChatTab: (chatId: string) => store.closeChatTab(chatId, t),
    openChat: () => store.openChat(t),
    closeChat: () => store.closeChat(),
    minimizeChat: () => store.minimizeChat(),
    maximizeChat: () => store.maximizeChat(),
    setLoading: (loading: boolean) => store.setLoading(loading),
  };
}

/** Reset module init flags (tests only). */
export function __resetSvedaChatHookForTests(): void {
  initialized = false;
}
