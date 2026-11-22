<script setup>
  import { buildAssistantMessageSegments, collectResourceLinks } from '@sveda-ai/chat';
  import { renderMarkdown } from '@sveda-ai/chat';
  import { useAnimatedText } from '../../lib/useAnimatedText.js';
  import { getUserMessageAttachmentNames, getUserMessageText } from '@sveda-ai/chat';
  import { useSvedaT } from '../../i18n/index.js';
  import { Paperclip } from 'lucide-vue-next';
  import { computed } from 'vue';
  import { Button } from '../../ui/button/index.js';
  import ChatMessageActivityBlock from './ChatMessageActivityBlock.vue';
  import ChatToolConfirmation from './ChatToolConfirmation.vue';
  import ReasoningStream from './ReasoningStream.vue';
  import { needsToolConfirmation } from './chatToolActivity.js';

  const t = useSvedaT();

  const props = defineProps({
    message: {
      type: Object,
      required: true,
    },
    approvalDisabled: Boolean,
    contentAnimationsEnabled: {
      type: Boolean,
      default: false,
    },
    onResourceLinkClick: {
      type: Function,
      default: null,
    },
  });

  defineEmits(['confirm-tool', 'deny-tool', 'approve-tool-batch', 'activity-link-click']);

  const messageSegments = computed(() => {
    if (props.message.role !== 'assistant') {
      return [];
    }
    return buildAssistantMessageSegments(props.message);
  });

  const reasoning = computed(() => messageSegments.value.filter(segment => segment.kind === 'reasoning').at(-1));
  const textSegments = computed(() => messageSegments.value.filter(segment => segment.kind === 'comment' || segment.kind === 'answer'));
  const activities = computed(() => messageSegments.value.filter(segment => segment.kind === 'activity'));
  const completedAndRunningTools = computed(() =>
    activities.value.flatMap(segment => segment.invocations.filter(invocation => !needsToolConfirmation(invocation)))
  );
  const legacyActivity = computed(() => activities.value.find(segment => segment.legacyActivity)?.legacyActivity);
  const approvalGroups = computed(() => activities.value.flatMap(segment => {
    const invocations = segment.invocations.filter(invocation => needsToolConfirmation(invocation) && invocation.toolCallId);
    if (invocations.length === 0) {
      return [];
    }
    const batchIds = invocations.filter(invocation =>
      invocation.confirmationScope === 'batch' || invocation.confirmationScope === 'chat'
    ).map(invocation => invocation.toolCallId);
    return [{ id: segment.id, invocations, batchIds }];
  }));

  const userMessageText = computed(() => (props.message.role === 'user' ? getUserMessageText(props.message) : ''));

  const displayedContent = useAnimatedText(() => userMessageText.value, {
    enabled: () => false,
  });

  const userAttachmentNames = computed(() => (props.message.role === 'user' ? getUserMessageAttachmentNames(props.message) : []));

  const hasUserText = computed(() => userMessageText.value.length > 0);

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
      if (!messageSegments.value.length && !isError) {
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
    <ReasoningStream v-if="reasoning" :text="reasoning.text" />

    <template v-for="segment in textSegments" :key="segment.id">
      <div :class="message.isError ? 'text-destructive' : ''">
        <div
          class="prose prose-sm dark:prose-invert max-w-none break-words text-sm text-foreground [overflow-wrap:anywhere] [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_pre]:overflow-x-auto [&_pre]:border [&_pre]:border-border [&_pre]:bg-muted [&_pre]:p-2 [&_pre]:font-mono [&_pre]:text-xs"
          @click="handleContentClick"
        >
          <div
            class="[&_a]:break-all [&_blockquote]:break-words [&_h1]:break-words [&_h2]:break-words [&_h3]:break-words [&_h4]:break-words [&_img]:my-2 [&_img]:h-auto [&_img]:max-w-full [&_img]:rounded-md [&_li]:break-words [&_p:last-child]:mb-0 [&_p]:my-1 [&_p]:break-words"
            v-html="renderSegmentMarkdown(segment.text, { injectResources: segment.kind === 'answer' })"
          ></div>
          <span
            v-if="showAnswerCursor(segment)"
            class="ml-1 inline-block h-4 w-2 animate-pulse bg-primary align-middle"
          ></span>
        </div>
      </div>
    </template>

    <ChatMessageActivityBlock
      :invocations="completedAndRunningTools"
      :legacy-activity="legacyActivity"
      :content-animations-enabled="contentAnimationsEnabled"
      @activity-link-click="$emit('activity-link-click', $event)"
    />

    <div v-for="group in approvalGroups" :key="group.id" class="grid gap-2.5">
      <ChatToolConfirmation
        v-for="invocation in group.invocations"
        :key="invocation.toolCallId"
        :invocation="invocation"
        :disabled="approvalDisabled"
        @confirm-tool="(id, decision) => $emit('confirm-tool', id, decision)"
        @deny-tool="$emit('deny-tool', $event)"
      />
      <Button
        v-if="group.batchIds.length > 1"
        type="button"
        size="sm"
        variant="outline"
        class="max-w-full justify-self-start whitespace-normal text-xs"
        :disabled="approvalDisabled"
        @click="$emit('approve-tool-batch', group.batchIds)"
      >
        {{ t('toolApproveBatch', { count: group.batchIds.length }) }}
      </Button>
    </div>

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
</style>
