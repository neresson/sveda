<script setup>
  import SvedaContextUsageBar from './SvedaContextUsageBar.vue';
  import SvedaModelControls from './SvedaModelControls.vue';
  import ChatInput from '../chat/ChatInput.vue';

  const inputMessage = defineModel('inputMessage', { type: String, default: '' });
  const pendingFiles = defineModel('pendingFiles', { type: Array, default: () => [] });
  const selectedChatModel = defineModel('selectedChatModel', { type: String, required: true });
  const thinkingEnabled = defineModel('thinkingEnabled', { type: Boolean, default: true });

  defineProps({
    isLoading: { type: Boolean, default: false },
    confirmationPending: { type: Boolean, default: false },
    hasCurrentChat: { type: Boolean, default: false },
    isStreaming: { type: Boolean, default: false },
    statusBanner: { type: String, default: '' },
    modelSelectPlaceholder: { type: String, required: true },
    models: { type: Array, default: () => [] },
    thinkingTooltip: { type: String, required: true },
    contextWindowUsagePercent: { type: Number, required: true },
    contextWindowTooltip: { type: String, required: true },
  });

  const emit = defineEmits(['send', 'stop']);
</script>

<template>
  <ChatInput
    v-model="inputMessage"
    v-model:pending-files="pendingFiles"
    :is-loading="isLoading"
    :confirmation-pending="confirmationPending"
    :has-current-chat="hasCurrentChat"
    :is-streaming="isStreaming"
    :status-banner="statusBanner"
    @send="emit('send')"
    @stop="emit('stop')"
  >
    <template #bottom-left>
      <SvedaModelControls
        v-model="selectedChatModel"
        v-model:thinking-enabled="thinkingEnabled"
        :placeholder="modelSelectPlaceholder"
        :models="models"
        :thinking-tooltip="thinkingTooltip"
      />
    </template>
    <template #bottom-outside>
      <SvedaContextUsageBar
        :percent="contextWindowUsagePercent"
        :tooltip="contextWindowTooltip"
      />
    </template>
  </ChatInput>
</template>
