<script setup>
  import { ScrollArea } from '../../ui/scroll-area/index.js';
  import {
    assistantHasActiveTools,
    shouldShowChatPendingIndicator,
  } from '../../lib/chatMessageVisibility.js';
  import { useSvedaT } from '../../i18n/index.js';
  import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
  import ChatMessageItem from './ChatMessageItem.vue';
  import ChatPendingIndicator from './ChatPendingIndicator.vue';

  const props = defineProps({
    messages: {
      type: Array,
      required: true,
    },
    isLoading: {
      type: Boolean,
      default: false,
    },
    isThinking: {
      type: Boolean,
      default: false,
    },
    contentAnimationsEnabled: {
      type: Boolean,
      default: false,
    },
    thinkingMessage: {
      type: String,
      default: '',
    },
    pendingStatusMessages: {
      type: Array,
      default: () => [],
    },
    onResourceLinkClick: {
      type: Function,
      default: null,
    },
    noMtAuto: {
      type: Boolean,
      default: false,
    },
  });

  const emit = defineEmits(['near-bottom-change', 'scroll-to-bottom', 'stop-generation', 'activity-link-click']);

  const t = useSvedaT();
  const showPendingIndicator = computed(() => shouldShowChatPendingIndicator(props.isThinking && props.contentAnimationsEnabled, props.messages));
  const lastMessage = computed(() => props.messages[props.messages.length - 1]);
  const toolsActive = computed(() => assistantHasActiveTools(lastMessage.value));

  const resolvedPendingStatusMessages = computed(() => {
    const provided = props.pendingStatusMessages.filter(
      message => typeof message === 'string' && message.trim() !== ''
    );
    const fallback = [
      props.thinkingMessage,
      t('preparingReply'),
      t('analyzingRequest'),
      t('pendingStatus'),
      t('phaseToolCalling'),
    ].filter(message => typeof message === 'string' && message.trim() !== '' && message !== 'preparingReply' && message !== 'analyzingRequest' && message !== 'pendingStatus' && message !== 'phaseToolCalling');

    const messages = provided.length > 0 ? provided : fallback;
    if (!toolsActive.value) {
      return messages;
    }

    const tools = t('phaseToolCalling');
    if (!tools || tools === 'phaseToolCalling') {
      return messages;
    }
    return [tools, ...messages.filter(message => message !== tools)];
  });

  const messagesContainer = ref(null);
  const bottomRef = ref(null);
  const isNearBottom = ref(true);
  const SCROLL_BOTTOM_THRESHOLD = 48;
  let scrollViewport = null;

  const setIsNearBottom = value => {
    if (isNearBottom.value === value) {
      return;
    }

    isNearBottom.value = value;
    emit('near-bottom-change', value);
  };

  const getViewport = () => {
    const root = messagesContainer.value?.$el;
    if (!root) {
      return null;
    }

    return root.querySelector('[data-radix-scroll-area-viewport], [data-reka-scroll-area-viewport]') || root.firstElementChild;
  };

  const updateIsNearBottom = () => {
    const viewport = getViewport();
    if (!viewport) {
      setIsNearBottom(true);
      return;
    }

    const remainingScroll = viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight;
    setIsNearBottom(remainingScroll <= SCROLL_BOTTOM_THRESHOLD);
  };

  const detachScrollListener = () => {
    if (scrollViewport) {
      scrollViewport.removeEventListener('scroll', updateIsNearBottom);
      scrollViewport = null;
    }
  };

  const attachScrollListener = () => {
    detachScrollListener();

    const viewport = getViewport();
    if (!viewport) {
      return;
    }

    scrollViewport = viewport;
    scrollViewport.addEventListener('scroll', updateIsNearBottom, { passive: true });
    updateIsNearBottom();
  };

  const scrollToBottom = (options = {}) => {
    const behavior = options.behavior || (options.onlyIfNearBottom ? 'auto' : 'smooth');
    const applyScroll = () => {
      if (options.onlyIfNearBottom && !isNearBottom.value) {
        return;
      }

      const viewport = getViewport();
      if (viewport) {
        if (behavior === 'auto') {
          viewport.scrollTop = viewport.scrollHeight;
        } else {
          viewport.scrollTo({
            top: viewport.scrollHeight,
            behavior,
          });
        }
        setIsNearBottom(true);
      } else if (bottomRef.value) {
        bottomRef.value.scrollIntoView({ behavior, block: 'end' });
        setIsNearBottom(true);
      }
    };

    nextTick(() => {
      applyScroll();
      requestAnimationFrame(applyScroll);
    });
  };

  onMounted(() => {
    nextTick(attachScrollListener);
  });

  onUnmounted(detachScrollListener);

  defineExpose({
    scrollToBottom,
  });
</script>

<template>
  <ScrollArea
    ref="messagesContainer"
    class="min-h-0 flex-1"
  >
    <div class="flex min-h-full flex-col px-4 py-4">
      <div
        class="flex w-full flex-col space-y-4"
        :class="{ 'mt-auto': !noMtAuto }"
      >
        <div
          v-for="(message, index) in messages"
          :key="message.id || index"
          :class="['flex', message.role === 'user' ? 'justify-end' : 'justify-start']"
        >
          <ChatMessageItem
            :message="message"
            :content-animations-enabled="contentAnimationsEnabled"
            :follow-workflow-stream="contentAnimationsEnabled && message.role === 'assistant' && index === messages.length - 1"
            :on-resource-link-click="onResourceLinkClick"
            @activity-link-click="$emit('activity-link-click', $event)"
            @confirm-tool="$emit('confirm-tool', $event)"
            @deny-tool="$emit('deny-tool', $event)"
            @stop-generation="$emit('stop-generation', $event)"
          />
        </div>

        <ChatPendingIndicator
          :active="showPendingIndicator"
          :status-messages="resolvedPendingStatusMessages"
        />

        <slot />
        <div ref="bottomRef" />
      </div>
    </div>
  </ScrollArea>
</template>
