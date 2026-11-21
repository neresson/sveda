<script setup>
  import { useSvedaT } from '../../i18n/index.js';

  defineProps({
    step: { type: Object, required: true },
    disabled: { type: Boolean, default: false },
  });
  const emit = defineEmits(['confirm-tool', 'deny-tool']);
  const t = useSvedaT();
</script>

<template>
  <div class="mt-1 border border-border bg-card px-2.5 py-2">
    <p class="text-xs leading-snug text-foreground">{{ t('toolConfirmationPrompt') }}</p>
    <pre v-if="step.detail" class="mt-1.5 max-h-24 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] leading-snug text-muted-foreground">{{ step.detail }}</pre>
    <div class="mt-2 flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-50"
        :disabled="disabled"
        @click.stop="emit('confirm-tool', step.toolCallId, 'approve')"
      >
        {{ t('toolConfirm') }}
      </button>
      <button
        v-if="step.confirmationScope === 'chat'"
        type="button"
        class="border border-border bg-card px-3 py-1.5 text-xs font-medium text-foreground disabled:opacity-50"
        :disabled="disabled"
        @click.stop="emit('confirm-tool', step.toolCallId, 'approve_for_chat')"
      >
        {{ t('toolApproveForChat') }}
      </button>
      <button
        type="button"
        class="border border-border bg-card px-3 py-1.5 text-xs font-medium text-foreground disabled:opacity-50"
        :disabled="disabled"
        @click.stop="emit('deny-tool', step.toolCallId)"
      >
        {{ t('toolDeny') }}
      </button>
    </div>
    <p v-if="step.confirmationScope === 'chat'" class="mt-1.5 text-xs text-muted-foreground">
      {{ t('toolChatApprovalScope') }}
    </p>
  </div>
</template>
