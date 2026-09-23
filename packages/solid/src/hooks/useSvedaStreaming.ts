import {
  createSvedaStreaming,
  type SvedaSendPayload,
  type SvedaStreamingOptions as HeadlessStreamingOptions,
  type SvedaStreamingStore as HeadlessStreamingStore,
} from '@sveda-ai/chat';
import type { SvedaDisplayMessage, SvedaSendOptions } from '@sveda-ai/core';
import type { SvedaContextUsageEvent, SvedaToolProgressEvent } from '@sveda-ai/protocol';
import { createMemo, createSignal, onCleanup, type Accessor } from 'solid-js';
import { useSvedaT } from '../i18n/index';
import { useSvedaContext } from '../provider';

export interface SvedaStreamingStore {
  currentChat: Accessor<{
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

export type { SvedaSendPayload };

export function useSvedaStreaming(
  store: SvedaStreamingStore,
  scrollToBottom: (options?: { behavior?: string; onlyIfNearBottom?: boolean }) => void,
  options: SvedaStreamingOptions = {},
) {
  const t = useSvedaT();
  const ctx = useSvedaContext();
  const beforeSend = ctx.beforeSend;

  const [isThinking, setIsThinking] = createSignal(false);
  const [thinkingMessage, setThinkingMessage] = createSignal('');
  const [streamingChatIds, setStreamingChatIds] = createSignal(new Set<string>());
  const [unreadChatIds, setUnreadChatIds] = createSignal(new Set<string>());

  const headlessStore: HeadlessStreamingStore = {
    getCurrentChat: () => store.currentChat(),
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

  const controller = createSvedaStreaming(ctx.client, headlessStore, scrollToBottom, headlessOptions);

  const sync = () => {
    const state = controller.getState();
    setIsThinking(state.isThinking);
    setThinkingMessage(state.thinkingMessage);
    setStreamingChatIds(state.streamingChatIds);
    setUnreadChatIds(state.unreadChatIds);
  };

  const unsubscribe = controller.subscribe(sync);
  sync();

  const stopWatch = controller.watchCurrentChatId(() => store.currentChat()?.id);

  const isStreaming = createMemo(() => controller.isStreamingFor(store.currentChat()?.id));

  onCleanup(() => {
    stopWatch();
    unsubscribe();
    controller.dispose();
  });

  return {
    sendMessage: controller.sendMessage,
    continueAfterMaxSteps: controller.continueAfterMaxSteps,
    resolveToolConfirmation: controller.resolveToolConfirmation,
    isStreaming,
    isThinking,
    thinkingMessage,
    stopStreaming: controller.stopStreaming,
    streamingChatIds,
    unreadChatIds,
    clearChatUnread: controller.clearChatUnread,
  };
}
