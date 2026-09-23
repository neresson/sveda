import {
  getSvedaChatStore,
  type SvedaChatHistory,
  type SvedaChatState,
  type SvedaChatStoreSnapshot,
} from '@sveda-ai/chat';
import { createEffect, createMemo, createSignal, onCleanup } from 'solid-js';
import { useSvedaT } from '../i18n/index';
import { SVEDA_EMBED_AUTH_EVENT, useSvedaContext } from '../provider';

export type { SvedaChatHistory, SvedaChatState };

let initialized = false;
let subscribed = false;

const store = getSvedaChatStore({ embedAuthEvent: SVEDA_EMBED_AUTH_EVENT });

export function useSvedaChat() {
  const ctx = useSvedaContext();
  const t = useSvedaT();

  const [snapshot, setSnapshot] = createSignal<SvedaChatStoreSnapshot>(store.getState());
  const [revision, setRevision] = createSignal(0);

  if (!subscribed) {
    subscribed = true;
    store.subscribe(() => {
      setSnapshot(store.getState());
      setRevision(v => v + 1);
    });
  }

  createEffect(() => {
    store.setClient(ctx.client);
    store.setTranslate(t);
  });

  if (!initialized) {
    initialized = true;
    store.initAuthListener();
    void store.loadHistories();
  }

  const chatState = createMemo(() => snapshot().chatState);
  const currentChat = createMemo(() => snapshot().currentChat);
  const chatHistories = createMemo(() => snapshot().chatHistories);
  const isOpen = createMemo(() => chatState().isOpen);
  const isMinimized = createMemo(() => chatState().isMinimized);
  const isLoading = createMemo(() => chatState().isLoading);
  const isLoadingChatHistory = createMemo(() => chatState().isLoadingChatHistory);
  const hasCurrentChat = createMemo(() => !!currentChat());

  const sortedHistories = createMemo(() => {
    revision();
    return store.sortedHistories();
  });

  const openChatTabs = createMemo(() => {
    revision();
    return store.openChatTabs();
  });

  onCleanup(() => {
    /* store is a process singleton; keep subscription alive across remounts */
  });

  return {
    chatState,
    currentChat,
    chatHistories,
    sortedHistories,
    openChatTabs,
    isOpen,
    isMinimized,
    isLoading,
    isLoadingChatHistory,
    hasCurrentChat,
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
