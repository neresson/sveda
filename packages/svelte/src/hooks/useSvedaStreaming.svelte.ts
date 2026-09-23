import {
  createSvedaStreaming,
  type SvedaSendPayload,
  type SvedaStreamingOptions as HeadlessStreamingOptions,
  type SvedaStreamingStore as HeadlessStreamingStore,
} from '@sveda-ai/chat';
import type { SvedaClient, SvedaDisplayMessage, SvedaSendOptions } from '@sveda-ai/core';
import type { SvedaContextUsageEvent, SvedaToolProgressEvent } from '@sveda-ai/protocol';
import { onDestroy } from 'svelte';
import { useSvedaContext, useSvedaT } from '../context.js';

export interface SvedaStreamingStore {
  getCurrentChat: () => {
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
  options: SvedaStreamingOptions = {},
) {
  const t = useSvedaT();
  const ctx = useSvedaContext();
  const client = ctx.client as SvedaClient;
  const beforeSend = ctx.beforeSend;

  let isThinking = $state(false);
  let thinkingMessage = $state('');
  let streamingChatIds = $state(new Set<string>());
  let unreadChatIds = $state(new Set<string>());
  let isStreaming = $state(false);

  const headlessStore: HeadlessStreamingStore = {
    getCurrentChat: () => store.getCurrentChat(),
    setChatMessages: store.setChatMessages,
    setChatTitle: store.setChatTitle,
    refreshChatTitleFromServer: store.refreshChatTitleFromServer,
    incrementChatTokens: store.incrementChatTokens,
    setChatContextWindowTokens: store.setChatContextWindowTokens,
  };

  const headlessOptions: HeadlessStreamingOptions = {
    ...options,
    translate: t,
    beforeSend,
  };

  const controller = createSvedaStreaming(client, headlessStore, scrollToBottom, headlessOptions);

  const sync = () => {
    const state = controller.getState();
    isThinking = state.isThinking;
    thinkingMessage = state.thinkingMessage;
    streamingChatIds = state.streamingChatIds;
    unreadChatIds = state.unreadChatIds;
    isStreaming = controller.isStreamingFor(store.getCurrentChat()?.id);
  };

  const unsubscribe = controller.subscribe(sync);
  sync();

  const pollCurrentChat = controller.watchCurrentChatId(() => store.getCurrentChat()?.id);

  const interval = setInterval(() => {
    pollCurrentChat();
    isStreaming = controller.isStreamingFor(store.getCurrentChat()?.id);
  }, 250);

  onDestroy(() => {
    clearInterval(interval);
    unsubscribe();
    controller.dispose();
  });

  return {
    sendMessage: controller.sendMessage,
    continueAfterMaxSteps: controller.continueAfterMaxSteps,
    resolveToolConfirmation: controller.resolveToolConfirmation,
    get isStreaming() {
      return isStreaming;
    },
    get isThinking() {
      return isThinking;
    },
    get thinkingMessage() {
      return thinkingMessage;
    },
    stopStreaming: controller.stopStreaming,
    get streamingChatIds() {
      return streamingChatIds;
    },
    get unreadChatIds() {
      return unreadChatIds;
    },
    clearChatUnread: controller.clearChatUnread,
  };
}
