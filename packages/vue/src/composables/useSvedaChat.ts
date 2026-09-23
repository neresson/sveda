import {
  getSvedaChatStore,
  type SvedaChatHistory,
  type SvedaChatState,
} from '@sveda-ai/chat';
import { computed, inject, reactive, readonly, ref, shallowRef } from 'vue';
import { useSvedaT } from '../i18n/index';
import { SvedaClientKey, SVEDA_EMBED_AUTH_EVENT } from '../plugin';

export type { SvedaChatHistory, SvedaChatState };

let initialized = false;
let subscribed = false;

const store = getSvedaChatStore({ embedAuthEvent: SVEDA_EMBED_AUTH_EVENT });
const initial = store.getState();
const chatState = reactive<SvedaChatState>({ ...initial.chatState });
const currentChat = shallowRef<SvedaChatHistory | null>(initial.currentChat);
const chatHistories = shallowRef<SvedaChatHistory[]>(initial.chatHistories);
const storeRevision = ref(0);

const syncFromStore = () => {
  const snapshot = store.getState();
  Object.assign(chatState, snapshot.chatState);
  currentChat.value = snapshot.currentChat;
  chatHistories.value = snapshot.chatHistories;
  storeRevision.value += 1;
};

if (!subscribed) {
  subscribed = true;
  store.subscribe(syncFromStore);
}

export const useSvedaChat = () => {
  const client = inject(SvedaClientKey, null);
  const t = useSvedaT();

  if (client) {
    store.setClient(client);
  }
  store.setTranslate(t);

  if (!initialized) {
    initialized = true;
    store.initAuthListener();
    void store.loadHistories();
  }

  const isOpen = computed(() => chatState.isOpen);
  const isMinimized = computed(() => chatState.isMinimized);
  const isLoading = computed(() => chatState.isLoading);
  const isLoadingChatHistory = computed(() => chatState.isLoadingChatHistory);
  const hasCurrentChat = computed(() => !!currentChat.value);
  const sortedHistories = computed(() => {
    void storeRevision.value;
    return store.sortedHistories();
  });
  const openChatTabs = computed(() => {
    void storeRevision.value;
    return store.openChatTabs();
  });

  return {
    chatState: readonly(chatState),
    currentChat,
    chatHistories: readonly(chatHistories),
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
};
