<script setup>
  import VedaContextUsageBar from './VedaContextUsageBar.vue';
  import VedaModelControls from './VedaModelControls.vue';
  import VedaQuickPrompts from './VedaQuickPrompts.vue';
  import ChatInput from '../chat/ChatInput.vue';
  import { useVedaT } from '../../i18n/index';

  const t = useVedaT();

  defineProps({
    inputMessage: String,
    isLoading: Boolean,
    hasCurrentChat: Boolean,
    isStreaming: Boolean,
    pendingFiles: {
      type: Array,
      default: () => [],
    },
    selectedChatModel: String,
    thinkingEnabled: Boolean,
    contextWindowUsagePercent: Number,
    showQuickPromptButtons: Boolean,
    quickPromptButtons: Array,
    models: {
      type: Array,
      default: () => [],
    },
    thinkingTooltip: String,
    statusBanner: {
      type: String,
      default: '',
    },
  });

  const emit = defineEmits([
    'update:inputMessage',
    'update:selectedChatModel',
    'update:thinkingEnabled',
    'update:pendingFiles',
    'send',
    'stop',
    'sendQuickPrompt',
  ]);

  const onInputUpdate = val => emit('update:inputMessage', val);
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col justify-center overflow-y-auto px-4 py-8">
    <div class="flex w-full flex-col gap-3">
      <slot name="top"></slot>
      <ChatInput
        :model-value="inputMessage"
        :pending-files="pendingFiles"
        @update:model-value="onInputUpdate"
        @update:pending-files="val => emit('update:pendingFiles', val)"
        :is-loading="isLoading"
        :has-current-chat="hasCurrentChat"
        :is-streaming="isStreaming"
        :status-banner="statusBanner"
        no-border
        @send="$emit('send')"
        @stop="$emit('stop')"
      >
        <template #bottom-left>
          <VedaModelControls
            :model-value="selectedChatModel"
            @update:model-value="$emit('update:selectedChatModel', $event)"
            :thinking-enabled="thinkingEnabled"
            @update:thinking-enabled="$emit('update:thinkingEnabled', $event)"
            :placeholder="t('modelSelectPlaceholder')"
            :models="models"
            :thinking-tooltip="thinkingTooltip"
          />
        </template>
        <template #bottom-outside>
          <VedaContextUsageBar
            :percent="contextWindowUsagePercent"
            :tooltip="t('contextWindowPercent', { percent: contextWindowUsagePercent })"
          />
        </template>
      </ChatInput>
      <VedaQuickPrompts
        v-if="showQuickPromptButtons"
        class="px-4"
        :prompts="quickPromptButtons"
        :title="t('quickPromptsTitle')"
        :disabled="isLoading || isStreaming"
        @select="$emit('sendQuickPrompt', $event)"
      />
    </div>
  </div>
</template>
