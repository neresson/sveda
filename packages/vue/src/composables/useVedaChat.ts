import {
  createMessageId,
  type VedaChatHistorySummary,
  type VedaClient,
  type VedaDisplayMessage,
} from '@veda-ai/core';
import { computed, inject, reactive, readonly, ref } from 'vue';
import { useVedaT } from '../i18n/index';
import {
  mergeServerChatHistories,
  resolveMergedCurrentChat,
  upsertChatHistoryMessages,
} from '../lib/chatHistoryMerge';
import { readPersistedMinimized, writePersistedMinimized } from '../lib/chatUiStorage';
import {
  finalizeMessagesForDisplay,
  type VedaFinalizableMessage,
} from '../lib/finalizeMessages';
import { getUserMessageAttachmentNames, getUserMessageText } from '../lib/userMessage';
import { VedaClientKey } from '../plugin';

export interface VedaChatHistory {
  id: string;
  title: string;
  preview?: string;
  messages: VedaDisplayMessage[];
  tokensUsed: number;
  contextWindowTokens: number;
  createdAt: number;
  updatedAt: number;
  messagesLoaded: boolean;
}

export interface VedaChatState {
  isOpen: boolean;
  isMinimized: boolean;
  currentChatId: string | null;
  isLoading: boolean;
  isLoadingChatHistory: boolean;
}

const initialIsMinimized = readPersistedMinimized();

const chatState = reactive<VedaChatState>({
  isOpen: !initialIsMinimized,
  isMinimized: initialIsMinimized,
  currentChatId: null,
  isLoading: false,
  isLoadingChatHistory: false,
});

const chatHistories = ref<VedaChatHistory[]>([]);
const currentChat = ref<VedaChatHistory | null>(null);
const openChatTabIds = ref<string[]>([]);

let activeClient: VedaClient | null = null;
let initialized = false;

const hasUserRequests = (chat: VedaChatHistory): boolean => {
  if (typeof chat.preview === 'string' && chat.preview.trim().length > 0) {
    return true;
  }

  return chat.messages.some(message => {
    if (message.role !== 'user') {
      return false;
    }

    const body = getUserMessageText(message);
    const hasFiles = getUserMessageAttachmentNames(message).length > 0;
    return body.length > 0 || hasFiles;
  });
};

const isServerSummary = (chat: VedaChatHistory): boolean => !chat.messagesLoaded;

const shouldIncludeInHistoryList = (chat: VedaChatHistory): boolean => {
  return hasUserRequests(chat) || isServerSummary(chat);
};

const generateChatId = (): string => `chat_${createMessageId()}`;

const ensureChatTab = (chatId: string) => {
  if (!openChatTabIds.value.includes(chatId)) {
    openChatTabIds.value = [...openChatTabIds.value, chatId];
  }
};

const removeChatTab = (chatId: string) => {
  openChatTabIds.value = openChatTabIds.value.filter(id => id !== chatId);
};

const toTimestamp = (value: unknown): number => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === 'string') {
    const parsed = Date.parse(value);
    if (!Number.isNaN(parsed)) {
      return parsed;
    }
  }
  return Date.now();
};

const finalizeForDisplay = (messages: unknown): VedaDisplayMessage[] => {
  const list = Array.isArray(messages) ? (messages as VedaFinalizableMessage[]) : [];
  return finalizeMessagesForDisplay(list) as unknown as VedaDisplayMessage[];
};

const resolveServerHistoryId = (history: VedaChatHistorySummary): string => {
  const fromChatId = typeof history.chatId === 'string' ? history.chatId.trim() : '';
  if (fromChatId !== '') {
    return fromChatId;
  }

  const fromId = typeof history.id === 'string' ? history.id.trim() : '';
  return fromId;
};

const mapServerHistory = (
  history: VedaChatHistorySummary & { messages?: unknown },
  messagesLoaded: boolean
): VedaChatHistory => ({
  id: resolveServerHistoryId(history),
  title: history.title || '',
  preview: typeof history.preview === 'string' ? history.preview : '',
  messages: messagesLoaded ? finalizeForDisplay(history.messages) : [],
  tokensUsed: Number(history.tokensUsed) || 0,
  contextWindowTokens: Number(history.contextWindowTokens) || 0,
  createdAt: toTimestamp(history.createdAt),
  updatedAt: toTimestamp(history.updatedAt),
  messagesLoaded,
});

const historiesEndpointConfigured = (): boolean => Boolean(activeClient?.endpoints.histories);

const fetchHistoriesFromServer = async (): Promise<VedaChatHistory[]> => {
  if (!activeClient || !historiesEndpointConfigured()) {
    return [];
  }

  try {
    const histories = await activeClient.listHistories();
    return histories.map(history => mapServerHistory(history, false));
  } catch (err) {
    console.error('Error loading chat histories from server:', err);
    return [];
  }
};

const fetchChatHistoryFromServer = async (chatId: string): Promise<VedaChatHistory | null> => {
  if (!activeClient || !historiesEndpointConfigured()) {
    return null;
  }

  try {
    const detail = await activeClient.getHistory(chatId);
    return mapServerHistory(detail, true);
  } catch (err) {
    console.error('Error loading chat history from server:', err);
    return null;
  }
};

const deleteChatOnServer = async (chatId: string) => {
  if (!activeClient || !historiesEndpointConfigured()) {
    return;
  }

  try {
    await activeClient.deleteHistory(chatId);
  } catch (err) {
    console.error('Error deleting chat history from server:', err);
  }
};

const renameChatOnServer = async (chatId: string, title: string) => {
  if (!activeClient || !historiesEndpointConfigured()) {
    return;
  }

  try {
    await activeClient.renameHistory(chatId, title);
  } catch (err) {
    console.error('Error renaming chat history on server:', err);
  }
};

const buildWelcomeMessage = (text: string): VedaDisplayMessage => ({
  id: createMessageId(),
  role: 'assistant',
  parts: [{ type: 'text', text }],
  createdAt: new Date().toISOString(),
});

const loadHistories = async () => {
  const serverHistories = await fetchHistoriesFromServer();

  chatHistories.value = mergeServerChatHistories(serverHistories, currentChat.value);
  currentChat.value = resolveMergedCurrentChat(chatHistories.value, currentChat.value);
  if (currentChat.value) {
    chatState.currentChatId = currentChat.value.id;
  }

  if (chatHistories.value.length > 0 && !currentChat.value) {
    const latest = [...chatHistories.value].sort((a, b) => b.updatedAt - a.updatedAt)[0];
    await setCurrentChat(latest.id);
  }
};

const createNewChat = (translate?: (key: string) => string): string => {
  const chatId = generateChatId();
  const welcomeText = translate ? translate('welcomeMessage') : '';
  const newChatTitle = translate ? translate('newChat') : 'New Chat';

  const newChat: VedaChatHistory = {
    id: chatId,
    title: newChatTitle,
    preview: '',
    messages: welcomeText ? [buildWelcomeMessage(welcomeText)] : [],
    tokensUsed: 0,
    contextWindowTokens: 0,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    messagesLoaded: true,
  };

  chatHistories.value.unshift(newChat);
  ensureChatTab(chatId);
  void setCurrentChat(chatId, translate);

  return chatId;
};

const loadChatHistoryIfNeeded = async (chatId: string): Promise<boolean> => {
  const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
  if (chatIndex === -1) {
    return false;
  }

  const chat = chatHistories.value[chatIndex];
  if (chat.messagesLoaded) {
    return true;
  }

  const loadedHistory = await fetchChatHistoryFromServer(chatId);
  if (!loadedHistory) {
    return false;
  }

  chatHistories.value[chatIndex] = {
    ...loadedHistory,
    preview: chat.preview || loadedHistory.preview,
  };

  return true;
};

const setCurrentChat = async (chatId: string, translate?: (key: string) => string) => {
  chatState.isLoadingChatHistory = true;
  try {
    await loadChatHistoryIfNeeded(chatId);

    const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
    const chat = chatIndex === -1 ? undefined : chatHistories.value[chatIndex];
    if (chat) {
      chatHistories.value[chatIndex] = {
        ...chat,
        messages: finalizeForDisplay(chat.messages),
      };
      currentChat.value = chatHistories.value[chatIndex];
      chatState.currentChatId = chatId;
      ensureChatTab(chatId);
      if (currentChat.value.messages.length === 0 && translate) {
        const welcomeText = translate('welcomeMessage');
        if (welcomeText) {
          currentChat.value.messages.push(buildWelcomeMessage(welcomeText));
        }
      }
    }
  } finally {
    chatState.isLoadingChatHistory = false;
  }
};

const addMessage = (chatId: string, message: VedaDisplayMessage): VedaDisplayMessage | null => {
  const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
  if (chatIndex !== -1) {
    const chat = chatHistories.value[chatIndex];
    if (!message.createdAt) {
      message.createdAt = new Date().toISOString();
    }
    if (!message.id) {
      message.id = createMessageId();
    }
    chat.messages = [...chat.messages, message];
    chat.messagesLoaded = true;
    chat.updatedAt = Date.now();

    chatHistories.value = [...chatHistories.value];

    if (currentChat.value?.id === chatId) {
      currentChat.value = chatHistories.value[chatIndex];
    }
    return message;
  }
  return null;
};

const setChatTitle = (chatId: string, title: string) => {
  const normalizedTitle = title.trim();
  if (!normalizedTitle) {
    return;
  }

  const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
  if (chatIndex === -1) {
    return;
  }

  const chat = chatHistories.value[chatIndex];
  chat.title = normalizedTitle;
  chat.updatedAt = Date.now();

  chatHistories.value = [...chatHistories.value];

  if (currentChat.value?.id === chatId) {
    currentChat.value = chatHistories.value[chatIndex];
  }
};

const incrementChatTokens = (chatId: string, tokens: number) => {
  const normalizedTokens = Number(tokens);
  if (!Number.isFinite(normalizedTokens) || normalizedTokens <= 0) {
    return;
  }

  const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
  if (chatIndex !== -1) {
    const chat = chatHistories.value[chatIndex];
    chat.tokensUsed = Math.max(0, Number(chat.tokensUsed || 0) + Math.floor(normalizedTokens));
    chat.updatedAt = Date.now();

    chatHistories.value = [...chatHistories.value];

    if (currentChat.value?.id === chatId) {
      currentChat.value = chatHistories.value[chatIndex];
    }
  }
};

const setChatContextWindowTokens = (chatId: string, tokens: number) => {
  const normalizedTokens = Number(tokens);
  if (!Number.isFinite(normalizedTokens) || normalizedTokens < 0) {
    return;
  }

  const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
  if (chatIndex !== -1) {
    const chat = chatHistories.value[chatIndex];
    chat.contextWindowTokens = Math.max(0, Math.floor(normalizedTokens));

    chatHistories.value = [...chatHistories.value];

    if (currentChat.value?.id === chatId) {
      currentChat.value = chatHistories.value[chatIndex];
    }
  }
};

const updateMessage = (
  chatId: string,
  messageId: string,
  updates: Partial<VedaDisplayMessage>
) => {
  const chatIndex = chatHistories.value.findIndex(c => c.id === chatId);
  if (chatIndex !== -1) {
    const chat = chatHistories.value[chatIndex];
    const messageIndex = chat.messages.findIndex(m => m.id === messageId);
    if (messageIndex !== -1) {
      const updatedMessage = {
        ...chat.messages[messageIndex],
        ...updates,
      };
      chat.messages.splice(messageIndex, 1, updatedMessage);
      chat.updatedAt = Date.now();

      chatHistories.value = [...chatHistories.value];

      if (currentChat.value?.id === chatId) {
        currentChat.value = chatHistories.value[chatIndex];
      }

      return updatedMessage;
    }
  }
  return null;
};

const setChatMessages = (chatId: string, messages: VedaDisplayMessage[]) => {
  const result = upsertChatHistoryMessages(
    chatHistories.value,
    currentChat.value,
    chatId,
    finalizeForDisplay(messages)
  );

  chatHistories.value = result.histories;
  currentChat.value = result.current;
  if (result.current?.id === chatId) {
    chatState.currentChatId = chatId;
    ensureChatTab(chatId);
  }
};

const deleteChat = (chatId: string, translate?: (key: string) => string) => {
  const index = chatHistories.value.findIndex(c => c.id === chatId);
  if (index !== -1) {
    chatHistories.value.splice(index, 1);
    removeChatTab(chatId);
    void deleteChatOnServer(chatId);

    if (chatState.currentChatId === chatId) {
      if (chatHistories.value.length > 0) {
        const nextChat = chatHistories.value[0];
        void setCurrentChat(nextChat.id, translate);
      } else {
        createNewChat(translate);
      }
    }
  }
};

const renameChat = (chatId: string, title: string) => {
  const normalizedTitle = title.trim();
  if (!normalizedTitle) {
    return;
  }

  setChatTitle(chatId, normalizedTitle);
  void renameChatOnServer(chatId, normalizedTitle);
};

export const useVedaChat = () => {
  const client = inject(VedaClientKey, null);
  const t = useVedaT();

  if (client) {
    activeClient = client;
  }

  if (!initialized) {
    initialized = true;
    if (historiesEndpointConfigured()) {
      void loadHistories();
    }
  }

  const isOpen = computed(() => chatState.isOpen);
  const isMinimized = computed(() => chatState.isMinimized);
  const isLoading = computed(() => chatState.isLoading);
  const isLoadingChatHistory = computed(() => chatState.isLoadingChatHistory);
  const hasCurrentChat = computed(() => !!currentChat.value);
  const sortedHistories = computed(() => {
    return [...chatHistories.value]
      .filter(shouldIncludeInHistoryList)
      .sort((a, b) => b.updatedAt - a.updatedAt);
  });
  const openChatTabs = computed(() => {
    const ids = [...openChatTabIds.value];
    if (currentChat.value?.id && !ids.includes(currentChat.value.id)) {
      ids.push(currentChat.value.id);
    }

    return ids
      .map(id => chatHistories.value.find(chat => chat.id === id))
      .filter((chat): chat is VedaChatHistory => !!chat);
  });

  const createNewChatWithI18n = (): string => {
    return createNewChat(t);
  };

  const setCurrentChatWithI18n = async (chatId: string) => {
    await setCurrentChat(chatId, t);
  };

  const closeChatTab = async (chatId: string) => {
    const currentIndex = openChatTabIds.value.findIndex(id => id === chatId);
    removeChatTab(chatId);

    if (currentChat.value?.id !== chatId) {
      return;
    }

    const nextChatId =
      openChatTabIds.value[currentIndex] ||
      openChatTabIds.value[currentIndex - 1] ||
      sortedHistories.value.find(chat => chat.id !== chatId)?.id;

    if (nextChatId) {
      await setCurrentChatWithI18n(nextChatId);
      return;
    }

    createNewChatWithI18n();
  };

  const openChat = async () => {
    chatState.isOpen = true;
    chatState.isMinimized = false;
    writePersistedMinimized(false);
    if (!currentChat.value && chatHistories.value.length === 0) {
      createNewChatWithI18n();
    } else if (!currentChat.value && chatHistories.value.length > 0) {
      await setCurrentChatWithI18n(chatHistories.value[0].id);
    }
  };

  const closeChat = () => {
    chatState.isOpen = false;
    chatState.isMinimized = true;
    writePersistedMinimized(true);
  };

  const minimizeChat = () => {
    chatState.isMinimized = true;
    writePersistedMinimized(true);
  };

  const maximizeChat = () => {
    chatState.isMinimized = false;
    chatState.isOpen = true;
    writePersistedMinimized(false);
  };

  const setLoading = (loading: boolean) => {
    chatState.isLoading = loading;
  };

  const refreshChatTitleFromServer = async (chatId: string): Promise<string | null> => {
    const loaded = await fetchChatHistoryFromServer(chatId);
    if (!loaded) {
      return null;
    }

    if (loaded.messages.length > 0) {
      const current =
        currentChat.value?.id === chatId
          ? currentChat.value
          : chatHistories.value.find(chat => chat.id === chatId);
      const localCount = current?.messages.length ?? 0;
      if (loaded.messages.length >= localCount) {
        setChatMessages(chatId, loaded.messages);
      }
    }

    const title = loaded.title?.trim();
    return title || null;
  };

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
    loadHistories,
    createNewChat: createNewChatWithI18n,
    setCurrentChat: setCurrentChatWithI18n,
    addMessage,
    setChatTitle,
    refreshChatTitleFromServer,
    updateMessage,
    setChatMessages,
    incrementChatTokens,
    setChatContextWindowTokens,
    deleteChat: (chatId: string) => deleteChat(chatId, t),
    renameChat: (chatId: string, title: string) => renameChat(chatId, title),
    closeChatTab,
    openChat,
    closeChat,
    minimizeChat,
    maximizeChat,
    setLoading,
  };
};
