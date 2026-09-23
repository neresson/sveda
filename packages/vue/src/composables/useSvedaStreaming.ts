import {
  createSvedaStreaming,
  type SvedaSendPayload,
  type SvedaStreamingOptions as HeadlessStreamingOptions,
  type SvedaStreamingStore as HeadlessStreamingStore,
} from '@sveda-ai/chat';
import type { SvedaClient, SvedaDisplayMessage, SvedaSendOptions } from '@sveda-ai/core';
import type { SvedaContextUsageEvent, SvedaToolProgressEvent } from '@sveda-ai/protocol';
import { computed, inject, onUnmounted, ref, watch, type Ref } from 'vue';
import { useSvedaT } from '../i18n/index';
import { SvedaBeforeSendKey, SvedaClientKey } from '../plugin';

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

export type { SvedaSendPayload };

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
  const beforeSend = inject(SvedaBeforeSendKey, null);

  const isThinking = ref(false);
  const thinkingMessage = ref('');
  const streamingChatIds = ref(new Set<string>());
  const unreadChatIds = ref(new Set<string>());

  const headlessStore: HeadlessStreamingStore = {
    getCurrentChat: () => store.currentChat.value,
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
    isThinking.value = state.isThinking;
    thinkingMessage.value = state.thinkingMessage;
    streamingChatIds.value = state.streamingChatIds;
    unreadChatIds.value = state.unreadChatIds;
  };

  const unsubscribe = controller.subscribe(sync);
  sync();

  const pollCurrentChat = controller.watchCurrentChatId(() => store.currentChat.value?.id);
  watch(
    () => store.currentChat.value?.id,
    () => {
      pollCurrentChat();
    }
  );

  const isStreaming = computed(() =>
    controller.isStreamingFor(store.currentChat.value?.id)
  );

  onUnmounted(() => {
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
