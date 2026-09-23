import type {
  SvedaChatSession,
  SvedaClient,
  SvedaDisplayMessage,
  SvedaMessagePart,
  SvedaSendOptions,
  SvedaSessionStatus,
} from '@sveda-ai/core';
import type { SvedaContextUsageEvent, SvedaMaxStepsEvent, SvedaToolProgressEvent } from '@sveda-ai/protocol';
import { deriveProvisionalChatTitle, isChatTitlePlaceholder } from './lib/chatTitle.js';

export interface SvedaStreamingChatRef {
  id: string;
  title?: string;
  messages?: SvedaDisplayMessage[];
}

export interface SvedaStreamingStore {
  getCurrentChat(): SvedaStreamingChatRef | null;
  setChatMessages(chatId: string, messages: SvedaDisplayMessage[]): void;
  setChatTitle(chatId: string, title: string): void;
  refreshChatTitleFromServer(chatId: string): Promise<string | null>;
  incrementChatTokens(chatId: string, tokens: number): void;
  setChatContextWindowTokens(chatId: string, tokens: number): void;
}

export interface SvedaStreamingOptions {
  newChatLabel?: () => string;
  resolveSendOptions?: () => Pick<SvedaSendOptions, 'model' | 'provider' | 'options'>;
  onMaxStepsReached?: (chatId: string, data: { maxSteps: number }) => void;
  onToolProgress?: (chatId: string, event: SvedaToolProgressEvent) => void;
  onContextUsage?: (chatId: string, event: SvedaContextUsageEvent) => void;
  onError?: (chatId: string, error: unknown) => void;
  onDone?: (chatId: string) => void;
  translate?: (key: string) => string;
  beforeSend?: (() => void | Promise<void>) | null;
  onAfterFinish?: (chatId: string) => void;
  onCurrentChatIdChange?: (newId: string | null | undefined, oldId: string | null | undefined) => void;
}

export type SvedaSendPayload =
  | string
  | {
      displayText: string;
      promptText: string;
      attachmentNames?: string[];
    };

export interface SvedaStreamingState {
  isThinking: boolean;
  thinkingMessage: string;
  streamingChatIds: Set<string>;
  unreadChatIds: Set<string>;
  statusByChatId: Record<string, SvedaSessionStatus>;
}

export interface SvedaStreamingController {
  getState(): SvedaStreamingState;
  subscribe(listener: () => void): () => void;
  sendMessage(userPayload: SvedaSendPayload, pageContext?: Record<string, unknown>): Promise<void>;
  continueAfterMaxSteps(pageContext?: Record<string, unknown>): Promise<void>;
  resolveToolConfirmation(
    toolCallId: string,
    decision: 'approve' | 'deny',
    pageContext?: Record<string, unknown>
  ): Promise<void>;
  stopStreaming(chatId?: string | null): void;
  clearChatUnread(chatId?: string | null): void;
  isStreamingFor(chatId: string | null | undefined): boolean;
  dispose(): void;
  watchCurrentChatId(getId: () => string | null | undefined): () => void;
}

export function createSvedaStreaming(
  client: SvedaClient,
  store: SvedaStreamingStore,
  scrollToBottom: (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => void,
  options: SvedaStreamingOptions = {}
): SvedaStreamingController {
  const listeners = new Set<() => void>();
  const translate = options.translate ?? ((key: string) => key);
  const beforeSend = options.beforeSend ?? null;

  let isThinking = false;
  let thinkingMessage = '';
  let streamingChatIds = new Set<string>();
  let unreadChatIds = new Set<string>();
  let statusByChatId: Record<string, SvedaSessionStatus> = {};

  const subscriptions = new Map<string, Array<() => void>>();
  const displayTextOverrides = new Map<string, Map<string, string>>();
  let lastWatchedChatId: string | null | undefined = undefined;

  const notify = () => {
    for (const listener of listeners) {
      listener();
    }
  };

  const getState = (): SvedaStreamingState => ({
    isThinking,
    thinkingMessage,
    streamingChatIds: new Set(streamingChatIds),
    unreadChatIds: new Set(unreadChatIds),
    statusByChatId: { ...statusByChatId },
  });

  const subscribe = (listener: () => void): (() => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };

  const setChatStreaming = (chatId: string, streaming: boolean) => {
    const next = new Set(streamingChatIds);
    if (streaming) {
      next.add(chatId);
    } else {
      next.delete(chatId);
    }
    streamingChatIds = next;
    notify();
  };

  const clearChatUnread = (chatId?: string | null) => {
    if (!chatId) {
      return;
    }
    const next = new Set(unreadChatIds);
    next.delete(chatId);
    unreadChatIds = next;
    notify();
  };

  const resolveNewChatLabel = () => {
    if (typeof options.newChatLabel === 'function') {
      return options.newChatLabel();
    }
    return translate('newChat');
  };

  const applyProvisionalChatTitle = (chatId: string, userRequest: string) => {
    const current = store.getCurrentChat();
    if (!current || current.id !== chatId) {
      return;
    }

    const newChatLabel = resolveNewChatLabel();
    if (!isChatTitlePlaceholder((current.title || '').trim(), newChatLabel)) {
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
    const previous = statusByChatId[chatId];
    statusByChatId = {
      ...statusByChatId,
      [chatId]: status,
    };

    const wasStreaming = previous === 'streaming' || previous === 'submitted';
    const isStreamingNow = status === 'streaming' || status === 'submitted';
    setChatStreaming(chatId, isStreamingNow);

    if (wasStreaming && !isStreamingNow && store.getCurrentChat()?.id !== chatId) {
      const next = new Set(unreadChatIds);
      next.add(chatId);
      unreadChatIds = next;
    }

    if (isStreamingNow) {
      clearChatUnread(chatId);
    }

    const currentId = store.getCurrentChat()?.id;
    if (chatId === currentId) {
      if (status === 'streaming' || status === 'submitted') {
        isThinking = true;
        thinkingMessage = translate('analyzingRequest');
      } else {
        isThinking = false;
        thinkingMessage = '';
      }
    }
    notify();
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
      queueMicrotask(() => {
        options.onDone?.(chatId);
        options.onAfterFinish?.(chatId);
        scheduleChatTitleRefresh(chatId);
        if (store.getCurrentChat()?.id === chatId) {
          scrollToBottom();
        }
      });
    });

    const offError = session.on('error', payload => {
      options.onError?.(chatId, payload);
      queueMicrotask(() => {
        options.onDone?.(chatId);
        if (store.getCurrentChat()?.id === chatId) {
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

  const syncCurrentChatMessages = (chatId: string | null | undefined) => {
    if (!chatId || !subscriptions.has(chatId)) {
      return;
    }

    const session = client.session(chatId);
    if (session.isStreaming) {
      return;
    }

    session.setMessages(store.getCurrentChat()?.messages ?? []);
  };

  const watchCurrentChatId = (getId: () => string | null | undefined): (() => void) => {
    lastWatchedChatId = getId();
    const poll = () => {
      const newId = getId();
      if (newId !== lastWatchedChatId) {
        const oldId = lastWatchedChatId;
        lastWatchedChatId = newId;
        options.onCurrentChatIdChange?.(newId, oldId);
        syncCurrentChatMessages(newId);

        const status = newId ? statusByChatId[newId] : undefined;
        if (status === 'streaming' || status === 'submitted') {
          isThinking = true;
          thinkingMessage = translate('analyzingRequest');
        } else {
          isThinking = false;
          thinkingMessage = '';
        }
        notify();
      }
    };

    // Store subscribers can call poll via returned handle; Vue adapter will drive via watch.
    return poll;
  };

  const isStreamingFor = (chatId: string | null | undefined): boolean =>
    streamingChatIds.has(chatId ?? '');

  const stopStreaming = (chatId: string | null | undefined = store.getCurrentChat()?.id) => {
    if (!chatId || !subscriptions.has(chatId)) {
      return;
    }
    client.session(chatId).stop();
    setChatStreaming(chatId, false);
    isThinking = false;
    thinkingMessage = '';
    notify();
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

    const current = store.getCurrentChat();
    if (!promptText || !current) {
      return;
    }

    try {
      await beforeSend?.();
    } catch (error) {
      options.onError?.(current.id, error);
      return;
    }

    const chatId = current.id;
    applyProvisionalChatTitle(chatId, promptText || displayText);
    const session = ensureSession(chatId);
    session.setMessages(store.getCurrentChat()?.messages ?? []);

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
    const chatId = store.getCurrentChat()?.id;
    if (!chatId) {
      return;
    }

    const session = ensureSession(chatId);
    if (session.messages.length === 0) {
      session.setMessages(store.getCurrentChat()?.messages ?? []);
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

  const resolveToolConfirmation = async (
    toolCallId: string,
    decision: 'approve' | 'deny',
    pageContext: Record<string, unknown> = {}
  ) => {
    const chatId = store.getCurrentChat()?.id;
    if (!chatId || !toolCallId) {
      return;
    }
    const session = ensureSession(chatId);
    session.setMessages(store.getCurrentChat()?.messages ?? []);
    try {
      await session.resolveToolConfirmation(toolCallId, decision, {
        ...resolveSendOptions(),
        context: { ...pageContext },
      });
    } catch {
      return;
    }
    scrollToBottom();
  };

  const dispose = () => {
    for (const [chatId, unsubscribes] of subscriptions.entries()) {
      client.session(chatId).stop();
      unsubscribes.forEach(off => off());
    }
    subscriptions.clear();
    displayTextOverrides.clear();
    listeners.clear();
  };

  return {
    getState,
    subscribe,
    sendMessage,
    continueAfterMaxSteps,
    resolveToolConfirmation,
    stopStreaming,
    clearChatUnread,
    isStreamingFor,
    dispose,
    watchCurrentChatId,
  };
}
