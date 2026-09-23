import {
  createSvedaStreaming,
  type SvedaSendPayload,
  type SvedaStreamingOptions as HeadlessStreamingOptions,
  type SvedaStreamingState,
  type SvedaStreamingStore as HeadlessStreamingStore,
} from '@sveda-ai/chat';
import type { SvedaDisplayMessage, SvedaSendOptions } from '@sveda-ai/core';
import type { SvedaContextUsageEvent, SvedaToolProgressEvent } from '@sveda-ai/protocol';
import { useEffect, useMemo, useRef, useSyncExternalStore } from 'react';
import { useSvedaContext, useSvedaT } from '../provider';

export interface SvedaStreamingStore {
  currentChat: {
    id: string;
    title?: string;
    messages?: SvedaDisplayMessage[];
  } | null;
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

export type { SvedaSendPayload };

export function useSvedaStreaming(
  store: SvedaStreamingStore,
  scrollToBottom: (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => void,
  options: SvedaStreamingOptions = {}
) {
  const t = useSvedaT();
  const { client, beforeSend } = useSvedaContext();
  const storeRef = useRef(store);
  storeRef.current = store;
  const scrollRef = useRef(scrollToBottom);
  scrollRef.current = scrollToBottom;
  const optionsRef = useRef(options);
  optionsRef.current = options;
  const translateRef = useRef(t);
  translateRef.current = t;

  const controller = useMemo(() => {
    const headlessStore: HeadlessStreamingStore = {
      getCurrentChat: () => storeRef.current.currentChat,
      setChatMessages: (chatId, messages) => storeRef.current.setChatMessages(chatId, messages),
      setChatTitle: (chatId, title) => storeRef.current.setChatTitle(chatId, title),
      refreshChatTitleFromServer: (chatId) => storeRef.current.refreshChatTitleFromServer(chatId),
      incrementChatTokens: (chatId, tokens) => storeRef.current.incrementChatTokens(chatId, tokens),
      setChatContextWindowTokens: (chatId, tokens) =>
        storeRef.current.setChatContextWindowTokens(chatId, tokens),
    };

    const headlessOptions: HeadlessStreamingOptions = {
      get newChatLabel() {
        return optionsRef.current.newChatLabel;
      },
      get resolveSendOptions() {
        return optionsRef.current.resolveSendOptions;
      },
      get onMaxStepsReached() {
        return optionsRef.current.onMaxStepsReached;
      },
      get onToolProgress() {
        return optionsRef.current.onToolProgress;
      },
      get onContextUsage() {
        return optionsRef.current.onContextUsage;
      },
      get onError() {
        return optionsRef.current.onError;
      },
      get onDone() {
        return optionsRef.current.onDone;
      },
      translate: (key: string) => translateRef.current(key),
      beforeSend,
    };

    return createSvedaStreaming(
      client,
      headlessStore,
      (opts) => scrollRef.current(opts),
      headlessOptions
    );
  }, [client, beforeSend]);

  const cachedState = useRef<SvedaStreamingState>(controller.getState());

  const streamingState = useSyncExternalStore(
    (listener) =>
      controller.subscribe(() => {
        cachedState.current = controller.getState();
        listener();
      }),
    () => cachedState.current,
    () => cachedState.current
  );

  const pollRef = useRef<() => void>(() => {});

  useEffect(() => {
    cachedState.current = controller.getState();
    pollRef.current = controller.watchCurrentChatId(() => storeRef.current.currentChat?.id);
    return () => {
      controller.dispose();
    };
  }, [controller]);

  useEffect(() => {
    pollRef.current();
  }, [store.currentChat?.id]);

  const isStreaming = controller.isStreamingFor(store.currentChat?.id);

  return {
    sendMessage: controller.sendMessage.bind(controller),
    continueAfterMaxSteps: controller.continueAfterMaxSteps.bind(controller),
    resolveToolConfirmation: controller.resolveToolConfirmation.bind(controller),
    isStreaming,
    isThinking: streamingState.isThinking,
    thinkingMessage: streamingState.thinkingMessage,
    stopStreaming: controller.stopStreaming.bind(controller),
    streamingChatIds: streamingState.streamingChatIds,
    unreadChatIds: streamingState.unreadChatIds,
    clearChatUnread: controller.clearChatUnread.bind(controller),
  };
}
