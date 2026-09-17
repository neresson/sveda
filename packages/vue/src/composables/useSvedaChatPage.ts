import { computed, onMounted, reactive, ref, watch, type ComputedRef, type Ref } from 'vue';
import { useSvedaT } from '../i18n/index';
import { useSvedaAgentTasks, type SvedaAgentTasksPayload } from './useSvedaAgentTasks';
import { useSvedaBootstrap } from './useSvedaBootstrap';
import { useSvedaChat } from './useSvedaChat';
import { useSvedaChatLayout } from './useSvedaChatLayout';
import { useSvedaContextWindow, SVEDA_CHAT_MAX_CONTEXT_TOKENS } from './useSvedaContextWindow';
import { useSvedaDocuments } from './useSvedaDocuments';
import { useSvedaMaxStepsContinue } from './useSvedaMaxStepsContinue';
import { useSvedaMessaging } from './useSvedaMessaging';
import { useSvedaModel, type SvedaChatModelOption } from './useSvedaModel';
import { useSvedaPrompts, type SvedaQuickPromptItem } from './useSvedaPrompts';
import { useSvedaScroll, type SvedaScrollContainer } from './useSvedaScroll';
import { useSvedaShell, type SvedaBrandInfo } from './useSvedaShell';
import { useSvedaStreaming } from './useSvedaStreaming';

export const svedaErrorMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }

  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string' && message.trim()) {
      return message;
    }
  }

  return fallback;
};

export interface SvedaChatPageOptions {
  models?: ComputedRef<SvedaChatModelOption[]>;
  quickPrompts?: ComputedRef<SvedaQuickPromptItem[]>;
  brand?: ComputedRef<SvedaBrandInfo>;
  pageUrl?: ComputedRef<string>;
  pageContext?: ComputedRef<Record<string, unknown>>;
  onNavigate?: (url: string) => void;
  notify?: (kind: 'error', message: string) => void;
  agentTasksSubscribe?: (
    onPayload: (payload: SvedaAgentTasksPayload) => void
  ) => void | (() => void);
}

export type SvedaChatPageRefs = {
  messagesContainerRef: Ref<SvedaScrollContainer>;
  chatCardRef: Ref<unknown>;
};

export function useSvedaChatPage(options: SvedaChatPageOptions, elementRefs: SvedaChatPageRefs) {
  const t = useSvedaT();

  const chatStore = useSvedaChat();
  const {
    currentChat,
    sortedHistories,
    isMinimized,
    isLoading,
    isLoadingChatHistory,
    loadHistories,
    createNewChat,
    setCurrentChat,
    setChatTitle,
    refreshChatTitleFromServer,
    setChatMessages,
    incrementChatTokens,
    setChatContextWindowTokens,
    deleteChat,
    renameChat,
    maximizeChat,
    minimizeChat,
    setLoading,
  } = chatStore;

  const layout = useSvedaChatLayout(isMinimized);

  const model = useSvedaModel(options.models ?? computed(() => []));
  const documents = useSvedaDocuments({
    onError: message => options.notify?.('error', message),
  });
  const prompts = useSvedaPrompts(currentChat, options.quickPrompts ?? computed(() => []));

  const contextWindowMaxTokens = ref(SVEDA_CHAT_MAX_CONTEXT_TOKENS);
  const contextWindow = useSvedaContextWindow(currentChat, contextWindowMaxTokens);

  const activeChatId = computed(() => currentChat.value?.id ?? null);

  const { agentTasks, hasActiveAgentTasks, clearAgentTasks, mergeAgentTasksPayload } =
    useSvedaAgentTasks({ subscribe: options.agentTasksSubscribe });

  const { messagesContainerRef } = elementRefs;
  const showHistorySidebar = ref(false);

  const scrollBridge: {
    scrollToBottom?: (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => void;
  } = {};

  const maxStepsBridge: {
    mark: (chatId: string, data: { maxSteps: number }) => void;
  } = {
    mark: () => {},
  };

  const {
    sendMessage: sendMessageStreaming,
    continueAfterMaxSteps: continueStreamingAfterMaxSteps,
    isStreaming,
    isThinking,
    thinkingMessage,
    stopStreaming,
    streamingChatIds,
    unreadChatIds,
    clearChatUnread,
  } = useSvedaStreaming(
    {
      currentChat,
      setChatMessages,
      setChatTitle,
      refreshChatTitleFromServer,
      incrementChatTokens,
      setChatContextWindowTokens,
    },
    scrollOptions => scrollBridge.scrollToBottom?.(scrollOptions),
    {
      newChatLabel: () => t('newChat'),
      resolveSendOptions: model.resolveStreamingSendOptions,
      onMaxStepsReached: (chatId, data) => maxStepsBridge.mark(chatId, data),
      onToolProgress: (_chatId, event) => mergeAgentTasksPayload(event),
      onContextUsage: (_chatId, event) => {
        if (typeof event.maxTokens === 'number' && event.maxTokens > 0) {
          contextWindowMaxTokens.value = event.maxTokens;
        }
      },
      onError: (_chatId, error) => {
        options.notify?.('error', svedaErrorMessage(error, t('errorSendingMessage')));
      },
    }
  );

  const maxStepsContinue = useSvedaMaxStepsContinue(
    computed(() => currentChat.value?.id),
    isStreaming,
    isLoading
  );
  maxStepsBridge.mark = maxStepsContinue.markMaxStepsReached;

  const maxStepsContinueMessage = computed(() =>
    t('maxStepsReachedNotice', { limit: maxStepsContinue.maxStepsContinueLimit.value })
  );

  const scroll = useSvedaScroll(
    messagesContainerRef,
    computed(() => currentChat.value?.id),
    options.pageUrl ?? computed(() => ''),
    prompts.messages,
    isStreaming
  );
  scrollBridge.scrollToBottom = scroll.scrollToBottom;

  const pageContext = options.pageContext ?? computed(() => ({}));

  const messaging = useSvedaMessaging({
    isLoading,
    currentChat,
    pageContext,
    sendMessageStreaming,
    maximizeChat,
    setLoading,
    resetAgentCompletedNotice: scroll.resetAgentCompletedNotice,
    resetMaxStepsContinueNotice: maxStepsContinue.resetMaxStepsContinueNotice,
    extractingDocuments: documents.extractingDocuments,
    extractChatDocuments: documents.extractChatDocuments,
    buildChatDocumentSections: documents.buildChatDocumentSections,
    onDocumentExtractError: documents.showDocumentExtractError,
    clearAgentTasks,
  });

  const continueAfterMaxSteps = async () => {
    const chatId = currentChat.value?.id;
    if (!chatId || maxStepsContinue.maxStepsContinueDisabled.value) {
      return;
    }

    maxStepsContinue.resetMaxStepsContinueNotice(chatId);
    scroll.resetAgentCompletedNotice();
    clearAgentTasks();
    maximizeChat();

    await continueStreamingAfterMaxSteps(pageContext.value);
    scroll.scrollToBottom({ behavior: 'smooth', onlyIfNearBottom: false });
  };

  const shell = useSvedaShell(currentChat, options.brand ?? computed(() => ({})), layout);

  const isImmersiveLandingLayout = computed(
    () => layout.isImmersiveDesktop.value && !prompts.hasUserMessages.value
  );

  const chatInputStatusBanner = computed(() => documents.documentStatusBanner.value || '');

  const { loadChatBootstrap } = useSvedaBootstrap({
    loadHistories,
    createNewChat,
    setCurrentChat,
    currentChat,
    sortedHistories,
    scrollToBottom: scroll.scrollToBottom,
  });

  const handleNewChat = () => {
    clearAgentTasks();
    maxStepsContinue.resetMaxStepsContinueNotice();
    createNewChat();
  };

  const handleSelectChat = async (chatId: string) => {
    clearChatUnread(chatId);
    scroll.resetAgentCompletedNotice();
    maxStepsContinue.resetMaxStepsContinueNotice();
    await setCurrentChat(chatId);
    scroll.scrollToBottom({ behavior: 'auto', onlyIfNearBottom: false });
  };

  const showDeleteChatDialog = ref(false);
  const pendingDeleteChatId = ref<string | null>(null);

  const handleDeleteChat = (chatId: string) => {
    pendingDeleteChatId.value = chatId;
    showDeleteChatDialog.value = true;
  };

  const confirmDeleteChat = () => {
    const chatId = pendingDeleteChatId.value;
    showDeleteChatDialog.value = false;

    if (chatId) {
      deleteChat(chatId);
    }
  };

  const cancelDeleteChat = () => {
    showDeleteChatDialog.value = false;
  };

  watch(showDeleteChatDialog, open => {
    if (!open) {
      pendingDeleteChatId.value = null;
    }
  });

  const showRenameChatDialog = ref(false);
  const pendingRenameChatId = ref<string | null>(null);
  const pendingRenameChatTitle = ref('');

  const handleRenameChat = (chatId: string) => {
    const chat = sortedHistories.value.find(item => item.id === chatId);
    pendingRenameChatId.value = chatId;
    pendingRenameChatTitle.value = (chat?.title || '').trim();
    showRenameChatDialog.value = true;
  };

  const confirmRenameChat = () => {
    const chatId = pendingRenameChatId.value;
    const title = pendingRenameChatTitle.value.trim();
    showRenameChatDialog.value = false;

    if (chatId && title) {
      renameChat(chatId, title);
    }
  };

  const cancelRenameChat = () => {
    showRenameChatDialog.value = false;
  };

  watch(showRenameChatDialog, open => {
    if (!open) {
      pendingRenameChatId.value = null;
      pendingRenameChatTitle.value = '';
    }
  });

  const handleActivityLinkClick = async (link: string) => {
    if (typeof link !== 'string' || link.trim() === '') {
      return;
    }
    await layout.collapseImmersiveToDefaultFixedMode();
    if (options.onNavigate) {
      options.onNavigate(link);
      return;
    }
    window.location.assign(link);
  };

  onMounted(async () => {
    await loadChatBootstrap();
  });

  return reactive({
    t,
    showHistorySidebar,
    ...chatStore,
    ...layout,
    ...model,
    ...documents,
    ...prompts,
    ...contextWindow,
    ...scroll,
    ...messaging,
    ...shell,
    chatInputStatusBanner,
    isStreaming,
    isThinking,
    thinkingMessage,
    stopStreaming,
    streamingChatIds,
    unreadChatIds,
    showMaxStepsContinueNotice: maxStepsContinue.showMaxStepsContinueNotice,
    maxStepsContinueMessage,
    maxStepsContinueDisabled: maxStepsContinue.maxStepsContinueDisabled,
    continueAfterMaxSteps,
    isImmersiveLandingLayout,
    agentTasks,
    hasActiveAgentTasks,
    clearAgentTasks,
    handleNewChat,
    handleSelectChat,
    handleDeleteChat,
    showDeleteChatDialog,
    confirmDeleteChat,
    cancelDeleteChat,
    handleRenameChat,
    showRenameChatDialog,
    pendingRenameChatTitle,
    confirmRenameChat,
    cancelRenameChat,
    handleActivityLinkClick,
  });
}
