import {
  getSvedaChatStore,
  type SvedaChatHistory,
  type SvedaChatState,
  type SvedaChatStoreSnapshot,
} from '@sveda-ai/chat';
import { useCallback, useEffect, useMemo, useRef, useSyncExternalStore } from 'react';
import { useSvedaContext, useSvedaT } from '../provider';
import { SVEDA_EMBED_AUTH_EVENT } from '../types';

export type { SvedaChatHistory, SvedaChatState };

const store = getSvedaChatStore({ embedAuthEvent: SVEDA_EMBED_AUTH_EVENT });

let initialized = false;
let cachedSnapshot: SvedaChatStoreSnapshot = store.getState();

const subscribe = (onStoreChange: () => void) =>
  store.subscribe(() => {
    cachedSnapshot = store.getState();
    onStoreChange();
  });

const getSnapshot = () => cachedSnapshot;

export function useSvedaChat() {
  const { client } = useSvedaContext();
  const t = useSvedaT();
  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  const translateRef = useRef(t);
  translateRef.current = t;

  const translate = useCallback((key: string) => translateRef.current(key), []);

  useEffect(() => {
    store.setClient(client);
    store.setTranslate(translate);

    if (!initialized) {
      initialized = true;
      store.initAuthListener();
      void store.loadHistories();
    }
  }, [client, translate]);

  const sortedHistories = useMemo(() => store.sortedHistories(), [snapshot]);
  const openChatTabs = useMemo(() => store.openChatTabs(), [snapshot]);

  const createNewChat = useCallback(() => store.createNewChat(translate), [translate]);
  const setCurrentChat = useCallback(
    (chatId: string) => store.setCurrentChat(chatId, translate),
    [translate]
  );
  const deleteChat = useCallback((chatId: string) => store.deleteChat(chatId, translate), [translate]);
  const closeChatTab = useCallback(
    (chatId: string) => store.closeChatTab(chatId, translate),
    [translate]
  );
  const openChat = useCallback(() => store.openChat(translate), [translate]);

  return {
    chatState: snapshot.chatState,
    currentChat: snapshot.currentChat,
    chatHistories: snapshot.chatHistories,
    sortedHistories,
    openChatTabs,
    isOpen: snapshot.chatState.isOpen,
    isMinimized: snapshot.chatState.isMinimized,
    isLoading: snapshot.chatState.isLoading,
    isLoadingChatHistory: snapshot.chatState.isLoadingChatHistory,
    hasCurrentChat: Boolean(snapshot.currentChat),
    loadHistories: store.loadHistories.bind(store),
    createNewChat,
    setCurrentChat,
    addMessage: store.addMessage.bind(store),
    setChatTitle: store.setChatTitle.bind(store),
    refreshChatTitleFromServer: store.refreshChatTitleFromServer.bind(store),
    updateMessage: store.updateMessage.bind(store),
    setChatMessages: store.setChatMessages.bind(store),
    incrementChatTokens: store.incrementChatTokens.bind(store),
    setChatContextWindowTokens: store.setChatContextWindowTokens.bind(store),
    deleteChat,
    renameChat: store.renameChat.bind(store),
    closeChatTab,
    openChat,
    closeChat: store.closeChat.bind(store),
    minimizeChat: store.minimizeChat.bind(store),
    maximizeChat: store.maximizeChat.bind(store),
    setLoading: store.setLoading.bind(store),
  };
}
