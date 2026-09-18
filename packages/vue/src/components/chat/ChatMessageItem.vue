<script setup>
  import { buildAssistantMessageSegments, collectResourceLinks } from '../../lib/assistantMessage.js';
  import { renderMarkdown } from '../../lib/markdown.js';
  import { useAnimatedText } from '../../lib/useAnimatedText.js';
  import { getUserMessageAttachmentNames, getUserMessageText } from '../../lib/userMessage.js';
  import { useSvedaT } from '../../i18n/index.js';
  import { Paperclip } from 'lucide-vue-next';
  import { computed, nextTick, onUnmounted, ref, watch } from 'vue';
  import ChatMessageActivityBlock from './ChatMessageActivityBlock.vue';
  import ReasoningStream from './ReasoningStream.vue';

  const t = useSvedaT();

  const props = defineProps({
    message: {
      type: Object,
      required: true,
    },
    contentAnimationsEnabled: {
      type: Boolean,
      default: false,
    },
    followWorkflowStream: {
      type: Boolean,
      default: false,
    },
    onResourceLinkClick: {
      type: Function,
      default: null,
    },
  });

  defineEmits(['activity-link-click']);

  const messageSegments = computed(() => {
    if (props.message.role !== 'assistant') {
      return [];
    }
    return buildAssistantMessageSegments(props.message);
  });

  const workflowSegments = computed(() =>
    messageSegments.value.filter(segment => segment.kind === 'activity' || segment.kind === 'comment' || segment.kind === 'reasoning')
  );

  const answerSegments = computed(() => messageSegments.value.filter(segment => segment.kind === 'answer'));

  const userMessageText = computed(() => (props.message.role === 'user' ? getUserMessageText(props.message) : ''));

  const displayedContent = useAnimatedText(() => userMessageText.value, {
    enabled: () => false,
  });

  const userAttachmentNames = computed(() => (props.message.role === 'user' ? getUserMessageAttachmentNames(props.message) : []));

  const hasUserText = computed(() => userMessageText.value.length > 0);

  const hasSegmentText = computed(() =>
    messageSegments.value.some(segment => (segment.kind === 'comment' || segment.kind === 'answer' || segment.kind === 'reasoning') && segment.text.trim())
  );

  const hasActivity = computed(() => props.message.role === 'assistant' && messageSegments.value.some(segment => segment.kind === 'activity'));

  const isMessageStreaming = computed(() => {
    if (!props.contentAnimationsEnabled) {
      return false;
    }
    if (props.message.streaming === true) {
      return true;
    }
    return props.message.parts?.some(part => part.state === 'streaming') ?? false;
  });

  const shouldHideMessage = computed(() => {
    if (props.message.role === 'assistant') {
      const isError = props.message.isError;
      if (!hasSegmentText.value && !hasActivity.value && !isError) {
        return true;
      }
    }
    return false;
  });

  const resourceLinks = computed(() => {
    if (Array.isArray(props.message.resources) && props.message.resources.length > 0) {
      return props.message.resources;
    }
    return collectResourceLinks(props.message);
  });

  const renderSegmentMarkdown = (text, { injectResources = false } = {}) =>
    renderMarkdown(text, {
      resources: resourceLinks.value,
      injectResources: injectResources && !isMessageStreaming.value,
    });

  const handleContentClick = event => {
    const link = event.target.closest('.resource-inline-link');
    if (!link) {
      return;
    }
    if (typeof props.onResourceLinkClick === 'function') {
      event.preventDefault();
      event.stopPropagation();
      props.onResourceLinkClick(link.getAttribute('href'));
    }
  };

  const showAnswerCursor = segment => segment.streaming || (props.contentAnimationsEnabled && isMessageStreaming.value && segment.kind === 'answer');

  const workflowPanelRef = ref(null);
  const workflowContentRef = ref(null);
  let workflowResizeObserver = null;

  const shouldFollowWorkflowStream = computed(() => props.followWorkflowStream);

  const blockWorkflowPanelUserScroll = event => {
    if (shouldFollowWorkflowStream.value) {
      event.preventDefault();
    }
  };

  const workflowScrollSignature = computed(() =>
    JSON.stringify(
      workflowSegments.value.map(segment => {
        if (segment.kind === 'activity') {
          return {
            kind: segment.kind,
            tools: segment.invocations.map(inv => [inv.toolCallId, inv.state, typeof inv.output === 'string' ? inv.output.length : 0]),
          };
        }

        return {
          kind: segment.kind,
          length: segment.text?.length ?? 0,
          streaming: segment.streaming,
        };
      })
    )
  );

  const scrollWorkflowPanelToBottom = () => {
    const applyScroll = () => {
      const panel = workflowPanelRef.value;
      if (!panel) {
        return;
      }
      panel.scrollTop = Math.max(0, panel.scrollHeight - panel.clientHeight);
    };

    nextTick(() => {
      requestAnimationFrame(() => {
        applyScroll();
        requestAnimationFrame(applyScroll);
      });
    });
  };

  const stopWorkflowResizeObserver = () => {
    workflowResizeObserver?.disconnect();
    workflowResizeObserver = null;
  };

  const startWorkflowResizeObserver = () => {
    stopWorkflowResizeObserver();
    const content = workflowContentRef.value;
    if (!content) {
      return;
    }
    workflowResizeObserver = new ResizeObserver(() => {
      scrollWorkflowPanelToBottom();
    });
    workflowResizeObserver.observe(content);
  };

  watch(
    [workflowScrollSignature, shouldFollowWorkflowStream],
    ([, shouldFollowStream]) => {
      if (!shouldFollowStream || workflowSegments.value.length === 0) {
        return;
      }
      scrollWorkflowPanelToBottom();
    },
    { flush: 'post' }
  );

  watch(
    [shouldFollowWorkflowStream, () => workflowSegments.value.length],
    ([shouldFollowStream]) => {
      stopWorkflowResizeObserver();
      if (!shouldFollowStream) {
        return;
      }
      nextTick(() => {
        startWorkflowResizeObserver();
        scrollWorkflowPanelToBottom();
      });
    },
    { flush: 'post', immediate: true }
  );

  onUnmounted(() => {
    stopWorkflowResizeObserver();
  });
</script>

<template>
  <div
    v-if="shouldHideMessage"
    class="hidden"
  ></div>
  <div
    v-else-if="message.role === 'user'"
    class="max-w-[80%] border border-primary bg-primary p-3 text-primary-foreground"
  >
    <div
      v-if="userAttachmentNames.length"
          class="mb-2 flex flex-col gap-1.5 border-b border-primary-foreground/20 pb-2"
        >
          <p class="font-mono text-[11px] font-medium uppercase tracking-[0.14em] text-primary-foreground/80">
        {{ t('attachmentsLabel') }}
      </p>
      <div class="flex flex-wrap gap-1.5">
        <span
          v-for="(name, i) in userAttachmentNames"
          :key="`${name}-${i}`"
          class="inline-flex max-w-full items-center gap-1 border border-primary-foreground/20 bg-primary-foreground/10 px-2 py-1 text-xs [overflow-wrap:anywhere]"
        >
          <Paperclip class="h-3 w-3 shrink-0 opacity-90" />
          <span>{{ name }}</span>
        </span>
      </div>
    </div>
    <p
      v-if="hasUserText"
      class="whitespace-pre-wrap break-words text-sm [overflow-wrap:anywhere]"
    >
      {{ displayedContent }}
    </p>
  </div>
  <div
    v-else
    class="w-full max-w-full space-y-3"
  >
    <div
      v-if="workflowSegments.length > 0"
      ref="workflowPanelRef"
      class="assistant-workflow-panel max-h-[25dvh] touch-pan-y overflow-y-auto overflow-x-hidden border border-border bg-muted/40 p-2"
      :class="{ 'assistant-workflow-panel--locked': shouldFollowWorkflowStream }"
      @wheel="blockWorkflowPanelUserScroll"
      @touchmove="blockWorkflowPanelUserScroll"
    >
      <div
        ref="workflowContentRef"
        class="space-y-1"
      >
        <template
          v-for="segment in workflowSegments"
          :key="segment.id"
        >
          <ChatMessageActivityBlock
            v-if="segment.kind === 'activity'"
            :invocations="segment.invocations"
            :legacy-activity="segment.legacyActivity"
            :content-animations-enabled="contentAnimationsEnabled"
            @activity-link-click="$emit('activity-link-click', $event)"
          />

          <ReasoningStream
            v-else-if="segment.kind === 'reasoning'"
            :text="segment.text"
            :animated="contentAnimationsEnabled"
            @content-resize="scrollWorkflowPanelToBottom"
          />

          <div
            v-else-if="segment.kind === 'comment'"
            class="rounded-md border border-border/40 bg-muted/30 px-3 py-2"
          >
            <div
              class="prose prose-sm dark:prose-invert max-w-none break-words text-sm text-muted-foreground [overflow-wrap:anywhere] [&_code]:rounded [&_code]:bg-muted/60 [&_code]:px-1 [&_code]:py-0.5 [&_p:last-child]:mb-0 [&_p]:my-1"
              v-html="renderSegmentMarkdown(segment.text)"
            ></div>
          </div>
        </template>
      </div>
    </div>

    <template
      v-for="segment in answerSegments"
      :key="segment.id"
    >
      <div :class="message.isError ? 'text-destructive' : ''">
        <div
          class="prose prose-sm dark:prose-invert max-w-none break-words text-sm text-foreground [overflow-wrap:anywhere] [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_pre]:overflow-x-auto [&_pre]:border [&_pre]:border-border [&_pre]:bg-muted [&_pre]:p-2 [&_pre]:font-mono [&_pre]:text-xs"
          @click="handleContentClick"
        >
          <div
            class="[&_a]:break-all [&_blockquote]:break-words [&_h1]:break-words [&_h2]:break-words [&_h3]:break-words [&_h4]:break-words [&_img]:my-2 [&_img]:h-auto [&_img]:max-w-full [&_img]:rounded-md [&_li]:break-words [&_p:last-child]:mb-0 [&_p]:my-1 [&_p]:break-words"
            v-html="renderSegmentMarkdown(segment.text, { injectResources: true })"
          ></div>
          <span
            v-if="showAnswerCursor(segment)"
            class="ml-1 inline-block h-4 w-2 animate-pulse bg-primary align-middle"
          ></span>
        </div>
      </div>
    </template>

    <div
      v-if="message.isError && messageSegments.length === 0"
      class="text-sm text-destructive"
    >
      {{ message.content }}
    </div>
  </div>
</template>

<style scoped>
  :deep(.resource-inline-link) {
    color: hsl(var(--primary));
    text-decoration: underline;
    text-decoration-style: dotted;
    text-underline-offset: 2px;
    cursor: pointer;
    transition:
      color 0.15s ease,
      text-decoration-color 0.15s ease;
    font-weight: 500;
  }

  :deep(.resource-inline-link:hover) {
    color: hsl(var(--primary) / 0.8);
    text-decoration-style: solid;
  }

  :deep(.resource-link-icon) {
    display: inline-block;
    vertical-align: middle;
    margin-left: 2px;
    margin-top: -2px;
    opacity: 0.7;
    transition:
      opacity 0.15s ease,
      transform 0.15s ease;
  }

  :deep(.resource-inline-link:hover .resource-link-icon) {
    opacity: 1;
    transform: translate(1px, -1px);
  }

  .assistant-workflow-panel--locked {
    touch-action: none;
    overscroll-behavior: contain;
  }
</style>
