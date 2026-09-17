import type {
  SvedaChatSession,
  SvedaClient,
  SvedaDisplayMessage,
  SvedaMessagePart,
  SvedaSendOptions,
  SvedaSessionStatus,
} from '@sveda-ai/core';
import type { SvedaContextUsageEvent, SvedaMaxStepsEvent, SvedaToolProgressEvent } from '@sveda-ai/protocol';
import { computed, inject, nextTick, onUnmounted, ref, watch, type Ref } from 'vue';
import { useSvedaT } from '../i18n/index';
import { deriveProvisionalChatTitle, isChatTitlePlaceholder } from '../lib/chatTitle';
import { SvedaClientKey } from '../plugin';

export interface SvedaStreamingStore {
  currentChat: Ref<{
    id: string;
    title?: string;
    messages?: SvedaDisplayMessage[];
  } | null>;
  setChatMessages: (chatId: string, messages: SvedaDisplayMessage[]) => void;
  setChatTitle: (chatId: string, title: string) => void;
  refreshChatTitleFromServer: (chatId: string) => Promise<string | null>;
  incrementChatTokens: (chatId: string, tokens: number) => void;
  setChatContextWindowTokens: (chatId: string, tokens: number) => void;
}

export interface SvedaStreamingOptions {
  newChatLabel?: () => string;
  resolveSendOptions?: () => Pick<SvedaSendOptions, 'model' | 'provider' | 'options'>;
  onMaxStepsReached?: (chatId: string, data: { maxSteps: number }) => void;
  onToolProgress?: (chatId: string, event: SvedaToolProgressEvent) => void;
  onContextUsage?: (chatId: string, event: SvedaContextUsageEvent) => void;
  onError?: (chatId: string, error: unknown) => void;
  onDone?: (chatId: string) => void;
}

export type SvedaSendPayload =
  | string
  | {
      displayText: string;
      promptText: string;
      attachmentNames?: string[];
    };

export function useSvedaStreaming(
  store: SvedaStreamingStore,
  scrollToBottom: (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => void,
  options: SvedaStreamingOptions = {}
) {
  const t = useSvedaT();
  const client = inject(SvedaClientKey, null) as SvedaClient | null;
  if (!client) {
    throw new Error('[sveda] useSvedaStreaming requires the Sveda plugin to be installed.');
  }

  const isThinking = ref(false);
  const thinkingMessage = ref('');
  const streamingChatIds = ref(new Set<string>());
  const unreadChatIds = ref(new Set<string>());
  const statusByChatId = ref<Record<string, SvedaSessionStatus>>({});

  const subscriptions = new Map<string, Array<() => void>>();
  const displayTextOverrides = new Map<string, Map<string, string>>();

  const setChatStreaming = (chatId: string, streaming: boolean) => {
    const next = new Set(streamingChatIds.value);
    if (streaming) {
      next.add(chatId);
    } else {
      next.delete(chatId);
    }
    streamingChatIds.value = next;
  };

  const clearChatUnread = (chatId?: string | null) => {
    if (!chatId) {
      return;
    }
    const next = new Set(unreadChatIds.value);
    next.delete(chatId);
    unreadChatIds.value = next;
  };

  const resolveNewChatLabel = () => {
    if (typeof options.newChatLabel === 'function') {
      return options.newChatLabel();
    }
    return t('newChat');
  };

  const applyProvisionalChatTitle = (chatId: string, userRequest: string) => {
    if (!store.currentChat.value || store.currentChat.value.id !== chatId) {
      return;
    }

    const newChatLabel = resolveNewChatLabel();
    if (!isChatTitlePlaceholder((store.currentChat.value.title || '').trim(), newChatLabel)) {
      return;
    }

    const provisionalTitle = deriveProvisionalChatTitle(userRequest);
    if (provisionalTitle) {
      store.setChatTitle(chatId, provisionalTitle);
    }
  };

  const scheduleChatTitleRefresh = (chatId: string) => {
    const newChatLabel = resolveNewChatLabel();
    const delays = [800, 2500, 6000];

    delays.forEach(delay => {
      setTimeout(async () => {
        const title = await store.refreshChatTitleFromServer(chatId);
        if (!title || isChatTitlePlaceholder(title, newChatLabel)) {
          return;
        }
        store.setChatTitle(chatId, title);
      }, delay);
    });
  };

  const setChatStatus = (chatId: string, status: SvedaSessionStatus) => {
    const previous = statusByChatId.value[chatId];
    statusByChatId.value = {
      ...statusByChatId.value,
      [chatId]: status,
    };

    const wasStreaming = previous === 'streaming' || previous === 'submitted';
    const isStreamingNow = status === 'streaming' || status === 'submitted';
    setChatStreaming(chatId, isStreamingNow);

    if (wasStreaming && !isStreamingNow && store.currentChat.value?.id !== chatId) {
      const next = new Set(unreadChatIds.value);
      next.add(chatId);
      unreadChatIds.value = next;
    }

    if (isStreamingNow) {
      clearChatUnread(chatId);
    }
  };

  const applyDisplayOverrides = (
    chatId: string,
    messages: SvedaDisplayMessage[]
  ): SvedaDisplayMessage[] => {
    const overrides = displayTextOverrides.get(chatId);
    if (!overrides || overrides.size === 0) {
      return messages;
    }

    return messages.map(message => {
      const override = overrides.get(message.id);
      if (override === undefined || message.role !== 'user') {
        return message;
      }

      const fileParts = message.parts.filter(part => part.type === 'file');
      const parts: SvedaMessagePart[] = [
        ...(override ? [{ type: 'text', text: override } as SvedaMessagePart] : []),
        ...fileParts,
      ];

      return { ...message, parts };
    });
  };

  const ensureSession = (chatId: string): SvedaChatSession => {
    const session = client.session(chatId);
    if (subscriptions.has(chatId)) {
      return session;
    }

    const offMessages = session.on('messages', () => {
      store.setChatMessages(chatId, applyDisplayOverrides(chatId, session.messages));
    });

    const offStatus = session.on('status', payload => {
      setChatStatus(chatId, payload as SvedaSessionStatus);
    });

    const offTitle = session.on('title', payload => {
      if (typeof payload === 'string' && payload.trim()) {
        store.setChatTitle(chatId, payload);
      }
    });

    const offMaxSteps = session.on('maxSteps', payload => {
      const event = payload as SvedaMaxStepsEvent | undefined;
      options.onMaxStepsReached?.(chatId, {
        maxSteps: typeof event?.maxSteps === 'number' ? event.maxSteps : 0,
      });
    });

    const offContextUsage = session.on('contextUsage', payload => {
      const event = payload as SvedaContextUsageEvent | undefined;
      if (event && typeof event.usedTokens === 'number') {
        store.setChatContextWindowTokens(chatId, event.usedTokens);
      }
      if (event) {
        options.onContextUsage?.(chatId, event);
      }
    });

    const offToolProgress = session.on('toolProgress', payload => {
      options.onToolProgress?.(chatId, payload as SvedaToolProgressEvent);
    });

    const offFinish = session.on('finish', payload => {
      const event = payload as
        | { usage?: { totalTokens?: number; promptTokens?: number } }
        | undefined;
      if (event?.usage) {
        if (typeof event.usage.totalTokens === 'number') {
          store.incrementChatTokens(chatId, event.usage.totalTokens);
        }
        if (typeof event.usage.promptTokens === 'number') {
          store.setChatContextWindowTokens(chatId, event.usage.promptTokens);
        }
      }
      void nextTick(() => {
        options.onDone?.(chatId);
        scheduleChatTitleRefresh(chatId);
        if (store.currentChat.value?.id === chatId) {
          scrollToBottom();
        }
      });
    });

    const offError = session.on('error', payload => {
      options.onError?.(chatId, payload);
      void nextTick(() => {
        options.onDone?.(chatId);
        if (store.currentChat.value?.id === chatId) {
          scrollToBottom();
        }
      });
    });

    subscriptions.set(chatId, [
      offMessages,
      offStatus,
      offTitle,
      offMaxSteps,
      offContextUsage,
      offToolProgress,
      offFinish,
      offError,
    ]);
    setChatStatus(chatId, session.status);

    return session;
  };

  watch(
    () => store.currentChat.value?.id,
    (newId, oldId) => {
      if (!newId || newId === oldId || !subscriptions.has(newId)) {
        return;
      }

      const session = client.session(newId);
      if (session.isStreaming) {
        return;
      }

      session.setMessages(store.currentChat.value?.messages ?? []);
    }
  );

  watch(
    () => statusByChatId.value[store.currentChat.value?.id ?? ''],
    status => {
      if (status === 'streaming' || status === 'submitted') {
        isThinking.value = true;
        thinkingMessage.value = t('analyzingRequest');
      } else {
        isThinking.value = false;
        thinkingMessage.value = '';
      }
    },
    { immediate: true }
  );

  const isStreaming = computed(() =>
    streamingChatIds.value.has(store.currentChat.value?.id ?? '')
  );

  const stopStreaming = (chatId: string | null | undefined = store.currentChat.value?.id) => {
    if (!chatId || !subscriptions.has(chatId)) {
      return;
    }
    client.session(chatId).stop();
    setChatStreaming(chatId, false);
    isThinking.value = false;
    thinkingMessage.value = '';
  };

  const resolveSendOptions = (): SvedaSendOptions => ({
    ...(options.resolveSendOptions?.() ?? {}),
  });

  const sendMessage = async (
    userPayload: SvedaSendPayload,
    pageContext: Record<string, unknown> = {}
  ) => {
    let displayText = '';
    let promptText = '';
    let attachmentNames: string[] = [];

    if (typeof userPayload === 'string') {
      displayText = userPayload.trim();
      promptText = displayText;
    } else if (userPayload && typeof userPayload === 'object') {
      displayText = String(userPayload.displayText ?? '').trim();
      promptText = String(userPayload.promptText ?? '').trim();
      attachmentNames = Array.isArray(userPayload.attachmentNames)
        ? userPayload.attachmentNames.map(name => String(name).trim()).filter(Boolean)
        : [];
    } else {
      return;
    }

    if (!promptText || !store.currentChat.value) {
      return;
    }

    const chatId = store.currentChat.value.id;
    applyProvisionalChatTitle(chatId, promptText || displayText);
    const session = ensureSession(chatId);
    session.setMessages(store.currentChat.value.messages ?? []);

    const sendOptions: SvedaSendOptions = {
      ...resolveSendOptions(),
      context: { ...pageContext },
      ...(attachmentNames.length > 0
        ? { files: attachmentNames.map(name => ({ name })) }
        : {}),
    };

    const sendPromise = session.send(promptText, sendOptions);

    if (displayText !== promptText) {
      const userMessage = [...session.messages].reverse().find(message => message.role === 'user');
      if (userMessage) {
        if (!displayTextOverrides.has(chatId)) {
          displayTextOverrides.set(chatId, new Map());
        }
        displayTextOverrides.get(chatId)!.set(userMessage.id, displayText);
        store.setChatMessages(chatId, applyDisplayOverrides(chatId, session.messages));
      }
    }

    try {
      await sendPromise;
    } catch {
      return;
    }

    scrollToBottom();
  };

  const continueAfterMaxSteps = async (pageContext: Record<string, unknown> = {}) => {
    const chatId = store.currentChat.value?.id;
    if (!chatId) {
      return;
    }

    const session = ensureSession(chatId);
    if (session.messages.length === 0) {
      session.setMessages(store.currentChat.value?.messages ?? []);
    }

    try {
      await session.continueAfterMaxSteps({
        ...resolveSendOptions(),
        context: { ...pageContext },
      });
    } catch {
      return;
    }

    scrollToBottom();
  };

  onUnmounted(() => {
    for (const [chatId, unsubscribes] of subscriptions.entries()) {
      client.session(chatId).stop();
      unsubscribes.forEach(off => off());
    }
    subscriptions.clear();
    displayTextOverrides.clear();
  });

  return {
    sendMessage,
    continueAfterMaxSteps,
    isStreaming,
    isThinking,
    thinkingMessage,
    stopStreaming,
    streamingChatIds,
    unreadChatIds,
    clearChatUnread,
  };
}
