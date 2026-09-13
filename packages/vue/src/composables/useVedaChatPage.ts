import { computed, onMounted, reactive, ref, watch, type ComputedRef, type Ref } from 'vue';
import { useVedaT } from '../i18n/index';
import { useVedaAgentTasks, type VedaAgentTasksPayload } from './useVedaAgentTasks';
import { useVedaBootstrap } from './useVedaBootstrap';
import { useVedaChat } from './useVedaChat';
import { useVedaChatLayout } from './useVedaChatLayout';
import { useVedaContextWindow, VEDA_CHAT_MAX_CONTEXT_TOKENS } from './useVedaContextWindow';
import { useVedaDocuments } from './useVedaDocuments';
import { useVedaMaxStepsContinue } from './useVedaMaxStepsContinue';
import { useVedaMessaging } from './useVedaMessaging';
import { useVedaModel, type VedaChatModelOption } from './useVedaModel';
import { useVedaPrompts, type VedaQuickPromptItem } from './useVedaPrompts';
import { useVedaScroll, type VedaScrollContainer } from './useVedaScroll';
import { useVedaShell, type VedaBrandInfo } from './useVedaShell';
import { useVedaStreaming } from './useVedaStreaming';

export interface VedaChatPageOptions {
  models?: ComputedRef<VedaChatModelOption[]>;
  quickPrompts?: ComputedRef<VedaQuickPromptItem[]>;
  brand?: ComputedRef<VedaBrandInfo>;
  pageUrl?: ComputedRef<string>;
  pageContext?: ComputedRef<Record<string, unknown>>;
  onNavigate?: (url: string) => void;
  notify?: (kind: 'error', message: string) => void;
  agentTasksSubscribe?: (
    onPayload: (payload: VedaAgentTasksPayload) => void
  ) => void | (() => void);
}

export type VedaChatPageRefs = {
  messagesContainerRef: Ref<VedaScrollContainer>;
  chatCardRef: Ref<unknown>;
};

export function useVedaChatPage(options: VedaChatPageOptions, elementRefs: VedaChatPageRefs) {
  const t = useVedaT();

  const chatStore = useVedaChat();
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

  const layout = useVedaChatLayout(isMinimized);

  const model = useVedaModel(options.models ?? computed(() => []));
  const documents = useVedaDocuments({
    onError: message => options.notify?.('error', message),
  });
  const prompts = useVedaPrompts(currentChat, options.quickPrompts ?? computed(() => []));

  const contextWindowMaxTokens = ref(VEDA_CHAT_MAX_CONTEXT_TOKENS);
  const contextWindow = useVedaContextWindow(currentChat, contextWindowMaxTokens);

  const activeChatId = computed(() => currentChat.value?.id ?? null);

  const { agentTasks, hasActiveAgentTasks, clearAgentTasks, mergeAgentTasksPayload } =
    useVedaAgentTasks({ subscribe: options.agentTasksSubscribe });

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
  } = useVedaStreaming(
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
        const message =
          error instanceof Error && error.message ? error.message : t('errorSendingMessage');
        options.notify?.('error', message);
      },
    }
  );

  const maxStepsContinue = useVedaMaxStepsContinue(
    computed(() => currentChat.value?.id),
    isStreaming,
    isLoading
  );
  maxStepsBridge.mark = maxStepsContinue.markMaxStepsReached;

  const maxStepsContinueMessage = computed(() =>
    t('maxStepsReachedNotice', { limit: maxStepsContinue.maxStepsContinueLimit.value })
  );

  const scroll = useVedaScroll(
    messagesContainerRef,
    computed(() => currentChat.value?.id),
    options.pageUrl ?? computed(() => ''),
    prompts.messages,
    isStreaming
  );
  scrollBridge.scrollToBottom = scroll.scrollToBottom;

  const pageContext = options.pageContext ?? computed(() => ({}));

  const messaging = useVedaMessaging({
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

  const shell = useVedaShell(currentChat, options.brand ?? computed(() => ({})), layout);

  const isImmersiveLandingLayout = computed(
    () => layout.isImmersiveDesktop.value && !prompts.hasUserMessages.value
  );

  const chatInputStatusBanner = computed(() => documents.documentStatusBanner.value || '');

  const { loadChatBootstrap } = useVedaBootstrap({
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
