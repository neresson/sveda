import {
  createMessageId,
  type SvedaChatHistorySummary,
  type SvedaClient,
  type SvedaDisplayMessage,
} from '@sveda-ai/core';
import {
  mergeServerChatHistories,
  resolveMergedCurrentChat,
  upsertChatHistoryMessages,
} from './lib/chatHistoryMerge.js';
import { readPersistedMinimized, writePersistedMinimized } from './lib/chatUiStorage.js';
import {
  finalizeMessagesForDisplay,
  type SvedaFinalizableMessage,
} from './lib/finalizeMessages.js';
import { getUserMessageAttachmentNames, getUserMessageText } from './lib/userMessage.js';

export interface SvedaChatHistory {
  id: string;
  title: string;
  preview?: string;
  messages: SvedaDisplayMessage[];
  tokensUsed: number;
  contextWindowTokens: number;
  createdAt: number;
  updatedAt: number;
  messagesLoaded: boolean;
}

export interface SvedaChatState {
  isOpen: boolean;
  isMinimized: boolean;
  currentChatId: string | null;
  isLoading: boolean;
  isLoadingChatHistory: boolean;
}

export interface SvedaChatStoreSnapshot {
  chatState: SvedaChatState;
  currentChat: SvedaChatHistory | null;
  chatHistories: SvedaChatHistory[];
  openChatTabIds: string[];
}

export type SvedaChatTranslate = (key: string) => string;

export interface SvedaChatStoreOptions {
  embedAuthEvent?: string;
}

export interface SvedaChatStore {
  getState(): SvedaChatStoreSnapshot;
  subscribe(listener: () => void): () => void;
  setClient(client: SvedaClient | null): void;
  getClient(): SvedaClient | null;
  setTranslate(translate: SvedaChatTranslate | undefined): void;
  loadHistories(): Promise<void>;
  createNewChat(translate?: SvedaChatTranslate): string;
  setCurrentChat(chatId: string, translate?: SvedaChatTranslate): Promise<void>;
  addMessage(chatId: string, message: SvedaDisplayMessage): SvedaDisplayMessage | null;
  setChatTitle(chatId: string, title: string): void;
  refreshChatTitleFromServer(chatId: string): Promise<string | null>;
  updateMessage(
    chatId: string,
    messageId: string,
    updates: Partial<SvedaDisplayMessage>
  ): SvedaDisplayMessage | null;
  setChatMessages(chatId: string, messages: SvedaDisplayMessage[]): void;
  incrementChatTokens(chatId: string, tokens: number): void;
  setChatContextWindowTokens(chatId: string, tokens: number): void;
  deleteChat(chatId: string, translate?: SvedaChatTranslate): void;
  renameChat(chatId: string, title: string): void;
  closeChatTab(chatId: string, translate?: SvedaChatTranslate): Promise<void>;
  openChat(translate?: SvedaChatTranslate): Promise<void>;
  closeChat(): void;
  minimizeChat(): void;
  maximizeChat(): void;
  setLoading(loading: boolean): void;
  sortedHistories(): SvedaChatHistory[];
  openChatTabs(): SvedaChatHistory[];
  initAuthListener(): void;
}

const hasUserRequests = (chat: SvedaChatHistory): boolean => {
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

const isServerSummary = (chat: SvedaChatHistory): boolean => !chat.messagesLoaded;

const shouldIncludeInHistoryList = (chat: SvedaChatHistory): boolean =>
  hasUserRequests(chat) || isServerSummary(chat);

const generateChatId = (): string => `chat_${createMessageId()}`;

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

const finalizeForDisplay = (messages: unknown): SvedaDisplayMessage[] => {
  const list = Array.isArray(messages) ? (messages as SvedaFinalizableMessage[]) : [];
  return finalizeMessagesForDisplay(list) as unknown as SvedaDisplayMessage[];
};

const resolveServerHistoryId = (history: SvedaChatHistorySummary): string => {
  const fromChatId = typeof history.chatId === 'string' ? history.chatId.trim() : '';
  if (fromChatId !== '') {
    return fromChatId;
  }

  const fromId = typeof history.id === 'string' ? history.id.trim() : '';
  return fromId;
};

const mapServerHistory = (
  history: SvedaChatHistorySummary & { messages?: unknown },
  messagesLoaded: boolean
): SvedaChatHistory => ({
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

const buildWelcomeMessage = (text: string): SvedaDisplayMessage => ({
  id: createMessageId(),
  role: 'assistant',
  parts: [{ type: 'text', text }],
  createdAt: new Date().toISOString(),
});

export function createSvedaChatStore(options: SvedaChatStoreOptions = {}): SvedaChatStore {
  const embedAuthEvent = options.embedAuthEvent ?? 'sveda-embed-auth';
  const listeners = new Set<() => void>();

  const initialIsMinimized = readPersistedMinimized();
  const chatState: SvedaChatState = {
    isOpen: !initialIsMinimized,
    isMinimized: initialIsMinimized,
    currentChatId: null,
    isLoading: false,
    isLoadingChatHistory: false,
  };

  let currentChat: SvedaChatHistory | null = null;
  let chatHistories: SvedaChatHistory[] = [];
  let openChatTabIds: string[] = [];
  let activeClient: SvedaClient | null = null;
  let translateFn: SvedaChatTranslate | undefined;
  let authListenerBound = false;

  const notify = () => {
    for (const listener of listeners) {
      listener();
    }
  };

  const getState = (): SvedaChatStoreSnapshot => ({
    chatState: { ...chatState },
    currentChat,
    chatHistories,
    openChatTabIds: [...openChatTabIds],
  });

  const subscribe = (listener: () => void): (() => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };

  const ensureChatTab = (chatId: string) => {
    if (!openChatTabIds.includes(chatId)) {
      openChatTabIds = [...openChatTabIds, chatId];
    }
  };

  const removeChatTab = (chatId: string) => {
    openChatTabIds = openChatTabIds.filter(id => id !== chatId);
  };

  const historiesEndpointConfigured = (): boolean => Boolean(activeClient?.endpoints.histories);

  const embedAuthReady = (): boolean => {
    if (!activeClient || activeClient.credentials !== 'omit') {
      return true;
    }

    const headers = activeClient.resolveHeaders();
    return Boolean(
      headers['X-Sveda-Embed-Token'] ||
        headers['x-sveda-embed-token'] ||
        headers.Authorization ||
        headers.authorization
    );
  };

  const fetchHistoriesFromServer = async (): Promise<SvedaChatHistory[]> => {
    if (!activeClient || !historiesEndpointConfigured() || !embedAuthReady()) {
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

  const fetchChatHistoryFromServer = async (chatId: string): Promise<SvedaChatHistory | null> => {
    if (!activeClient || !historiesEndpointConfigured() || !embedAuthReady()) {
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
    if (!activeClient || !historiesEndpointConfigured() || !embedAuthReady()) {
      return;
    }

    try {
      await activeClient.deleteHistory(chatId);
    } catch (err) {
      console.error('Error deleting chat history from server:', err);
    }
  };

  const renameChatOnServer = async (chatId: string, title: string) => {
    if (!activeClient || !historiesEndpointConfigured() || !embedAuthReady()) {
      return;
    }

    try {
      await activeClient.renameHistory(chatId, title);
    } catch (err) {
      console.error('Error renaming chat history on server:', err);
    }
  };

  const sortedHistories = (): SvedaChatHistory[] =>
    [...chatHistories].filter(shouldIncludeInHistoryList).sort((a, b) => b.updatedAt - a.updatedAt);

  const openChatTabs = (): SvedaChatHistory[] => {
    const ids = [...openChatTabIds];
    if (currentChat?.id && !ids.includes(currentChat.id)) {
      ids.push(currentChat.id);
    }

    return ids
      .map(id => chatHistories.find(chat => chat.id === id))
      .filter((chat): chat is SvedaChatHistory => !!chat);
  };

  const loadChatHistoryIfNeeded = async (chatId: string): Promise<boolean> => {
    const chatIndex = chatHistories.findIndex(c => c.id === chatId);
    if (chatIndex === -1) {
      return false;
    }

    const chat = chatHistories[chatIndex];
    if (chat.messagesLoaded) {
      return true;
    }

    const loadedHistory = await fetchChatHistoryFromServer(chatId);
    if (!loadedHistory) {
      return false;
    }

    chatHistories[chatIndex] = {
      ...loadedHistory,
      preview: chat.preview || loadedHistory.preview,
    };
    chatHistories = [...chatHistories];
    notify();
    return true;
  };

  const setCurrentChat = async (chatId: string, translate?: SvedaChatTranslate) => {
    const t = translate ?? translateFn;
    chatState.isLoadingChatHistory = true;
    notify();
    try {
      await loadChatHistoryIfNeeded(chatId);

      const chatIndex = chatHistories.findIndex(c => c.id === chatId);
      const chat = chatIndex === -1 ? undefined : chatHistories[chatIndex];
      if (chat) {
        chatHistories[chatIndex] = {
          ...chat,
          messages: finalizeForDisplay(chat.messages),
        };
        chatHistories = [...chatHistories];
        currentChat = chatHistories[chatIndex];
        chatState.currentChatId = chatId;
        ensureChatTab(chatId);
        if (currentChat.messages.length === 0 && t) {
          const welcomeText = t('welcomeMessage');
          if (welcomeText) {
            currentChat.messages.push(buildWelcomeMessage(welcomeText));
          }
        }
      }
    } finally {
      chatState.isLoadingChatHistory = false;
      notify();
    }
  };

  const createNewChat = (translate?: SvedaChatTranslate): string => {
    const t = translate ?? translateFn;
    const chatId = generateChatId();
    const welcomeText = t ? t('welcomeMessage') : '';
    const newChatTitle = t ? t('newChat') : 'New Chat';

    const newChat: SvedaChatHistory = {
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

    chatHistories = [newChat, ...chatHistories];
    currentChat = newChat;
    chatState.currentChatId = chatId;
    ensureChatTab(chatId);
    notify();
    void setCurrentChat(chatId, t);

    return chatId;
  };

  const loadHistories = async () => {
    const serverHistories = await fetchHistoriesFromServer();

    chatHistories = mergeServerChatHistories(serverHistories, currentChat);
    currentChat = resolveMergedCurrentChat(chatHistories, currentChat);
    if (currentChat) {
      chatState.currentChatId = currentChat.id;
    }
    notify();

    if (chatHistories.length > 0 && !currentChat) {
      const latest = [...chatHistories].sort((a, b) => b.updatedAt - a.updatedAt)[0];
      await setCurrentChat(latest.id);
    }
  };

  const addMessage = (chatId: string, message: SvedaDisplayMessage): SvedaDisplayMessage | null => {
    const chatIndex = chatHistories.findIndex(c => c.id === chatId);
    if (chatIndex === -1) {
      return null;
    }

    const chat = chatHistories[chatIndex];
    if (!message.createdAt) {
      message.createdAt = new Date().toISOString();
    }
    if (!message.id) {
      message.id = createMessageId();
    }
    chat.messages = [...chat.messages, message];
    chat.messagesLoaded = true;
    chat.updatedAt = Date.now();
    chatHistories = [...chatHistories];

    if (currentChat?.id === chatId) {
      currentChat = chatHistories[chatIndex];
    }
    notify();
    return message;
  };

  const setChatTitle = (chatId: string, title: string) => {
    const normalizedTitle = title.trim();
    if (!normalizedTitle) {
      return;
    }

    const chatIndex = chatHistories.findIndex(c => c.id === chatId);
    if (chatIndex === -1) {
      return;
    }

    const chat = chatHistories[chatIndex];
    chat.title = normalizedTitle;
    chat.updatedAt = Date.now();
    chatHistories = [...chatHistories];

    if (currentChat?.id === chatId) {
      currentChat = chatHistories[chatIndex];
    }
    notify();
  };

  const incrementChatTokens = (chatId: string, tokens: number) => {
    const normalizedTokens = Number(tokens);
    if (!Number.isFinite(normalizedTokens) || normalizedTokens <= 0) {
      return;
    }

    const chatIndex = chatHistories.findIndex(c => c.id === chatId);
    if (chatIndex === -1) {
      return;
    }

    const chat = chatHistories[chatIndex];
    chat.tokensUsed = Math.max(0, Number(chat.tokensUsed || 0) + Math.floor(normalizedTokens));
    chat.updatedAt = Date.now();
    chatHistories = [...chatHistories];

    if (currentChat?.id === chatId) {
      currentChat = chatHistories[chatIndex];
    }
    notify();
  };

  const setChatContextWindowTokens = (chatId: string, tokens: number) => {
    const normalizedTokens = Number(tokens);
    if (!Number.isFinite(normalizedTokens) || normalizedTokens < 0) {
      return;
    }

    const chatIndex = chatHistories.findIndex(c => c.id === chatId);
    if (chatIndex === -1) {
      return;
    }

    const chat = chatHistories[chatIndex];
    chat.contextWindowTokens = Math.max(0, Math.floor(normalizedTokens));
    chatHistories = [...chatHistories];

    if (currentChat?.id === chatId) {
      currentChat = chatHistories[chatIndex];
    }
    notify();
  };

  const updateMessage = (
    chatId: string,
    messageId: string,
    updates: Partial<SvedaDisplayMessage>
  ): SvedaDisplayMessage | null => {
    const chatIndex = chatHistories.findIndex(c => c.id === chatId);
    if (chatIndex === -1) {
      return null;
    }

    const chat = chatHistories[chatIndex];
    const messageIndex = chat.messages.findIndex(m => m.id === messageId);
    if (messageIndex === -1) {
      return null;
    }

    const updatedMessage = {
      ...chat.messages[messageIndex],
      ...updates,
    };
    chat.messages.splice(messageIndex, 1, updatedMessage);
    chat.updatedAt = Date.now();
    chatHistories = [...chatHistories];

    if (currentChat?.id === chatId) {
      currentChat = chatHistories[chatIndex];
    }
    notify();
    return updatedMessage;
  };

  const setChatMessages = (chatId: string, messages: SvedaDisplayMessage[]) => {
    const result = upsertChatHistoryMessages(
      chatHistories,
      currentChat,
      chatId,
      finalizeForDisplay(messages)
    );

    chatHistories = result.histories;
    currentChat = result.current;
    if (result.current?.id === chatId) {
      chatState.currentChatId = chatId;
      ensureChatTab(chatId);
    }
    notify();
  };

  const deleteChat = (chatId: string, translate?: SvedaChatTranslate) => {
    const t = translate ?? translateFn;
    const index = chatHistories.findIndex(c => c.id === chatId);
    if (index === -1) {
      return;
    }

    chatHistories = chatHistories.filter(c => c.id !== chatId);
    removeChatTab(chatId);
    void deleteChatOnServer(chatId);
    notify();

    if (chatState.currentChatId === chatId) {
      if (chatHistories.length > 0) {
        void setCurrentChat(chatHistories[0].id, t);
      } else {
        createNewChat(t);
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

  const closeChatTab = async (chatId: string, translate?: SvedaChatTranslate) => {
    const t = translate ?? translateFn;
    const currentIndex = openChatTabIds.findIndex(id => id === chatId);
    removeChatTab(chatId);
    notify();

    if (currentChat?.id !== chatId) {
      return;
    }

    const nextChatId =
      openChatTabIds[currentIndex] ||
      openChatTabIds[currentIndex - 1] ||
      sortedHistories().find(chat => chat.id !== chatId)?.id;

    if (nextChatId) {
      await setCurrentChat(nextChatId, t);
      return;
    }

    createNewChat(t);
  };

  const openChat = async (translate?: SvedaChatTranslate) => {
    const t = translate ?? translateFn;
    chatState.isOpen = true;
    chatState.isMinimized = false;
    writePersistedMinimized(false);
    notify();
    if (!currentChat && chatHistories.length === 0) {
      createNewChat(t);
    } else if (!currentChat && chatHistories.length > 0) {
      await setCurrentChat(chatHistories[0].id, t);
    }
  };

  const closeChat = () => {
    chatState.isOpen = false;
    chatState.isMinimized = true;
    writePersistedMinimized(true);
    notify();
  };

  const minimizeChat = () => {
    chatState.isMinimized = true;
    writePersistedMinimized(true);
    notify();
  };

  const maximizeChat = () => {
    chatState.isMinimized = false;
    chatState.isOpen = true;
    writePersistedMinimized(false);
    notify();
  };

  const setLoading = (loading: boolean) => {
    chatState.isLoading = loading;
    notify();
  };

  const refreshChatTitleFromServer = async (chatId: string): Promise<string | null> => {
    const loaded = await fetchChatHistoryFromServer(chatId);
    if (!loaded) {
      return null;
    }

    if (loaded.messages.length > 0) {
      const current =
        currentChat?.id === chatId
          ? currentChat
          : chatHistories.find(chat => chat.id === chatId);
      const localCount = current?.messages.length ?? 0;
      if (loaded.messages.length >= localCount) {
        setChatMessages(chatId, loaded.messages);
      }
    }

    const title = loaded.title?.trim();
    return title || null;
  };

  const initAuthListener = () => {
    if (authListenerBound || typeof window === 'undefined') {
      return;
    }
    authListenerBound = true;
    window.addEventListener(embedAuthEvent, () => {
      if (historiesEndpointConfigured() && embedAuthReady()) {
        void loadHistories();
      }
    });
  };

  return {
    getState,
    subscribe,
    setClient(client) {
      activeClient = client;
    },
    getClient() {
      return activeClient;
    },
    setTranslate(translate) {
      translateFn = translate;
    },
    loadHistories,
    createNewChat,
    setCurrentChat,
    addMessage,
    setChatTitle,
    refreshChatTitleFromServer,
    updateMessage,
    setChatMessages,
    incrementChatTokens,
    setChatContextWindowTokens,
    deleteChat,
    renameChat,
    closeChatTab,
    openChat,
    closeChat,
    minimizeChat,
    maximizeChat,
    setLoading,
    sortedHistories,
    openChatTabs,
    initAuthListener,
  };
}

let sharedChatStore: SvedaChatStore | null = null;

export function getSvedaChatStore(options?: SvedaChatStoreOptions): SvedaChatStore {
  if (!sharedChatStore) {
    sharedChatStore = createSvedaChatStore(options);
  }
  return sharedChatStore;
}
