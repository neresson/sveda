<script setup>
  import { getToolNameFromPart } from '@sveda-ai/chat';
  import { ShieldCheck, TriangleAlert } from 'lucide-vue-next';
  import { computed, useId } from 'vue';
  import { useSvedaT } from '../../i18n/index.js';
  import { Button } from '../../ui/button/index.js';
  import { translateToolName } from './chatToolActivity.js';

  const props = defineProps({
    invocation: { type: Object, required: true },
    disabled: { type: Boolean, default: false },
  });
  const emit = defineEmits(['confirm-tool', 'deny-tool']);
  const t = useSvedaT();
  const titleId = useId();
  const toolName = computed(() => getToolNameFromPart(props.invocation));
  const action = computed(() => translateToolName(t, toolName.value) || t('toolConfirmationAction'));
  const destructive = computed(() => /^(delete|remove|bulk_delete|bulk_remove)_/.test(toolName.value));
  const scope = computed(() => props.invocation.confirmationScope);
  const supportsScopedApproval = computed(() => scope.value === 'chat' || scope.value === 'course');

  const targetName = computed(() => {
    const input = props.invocation.input ?? props.invocation.args ?? {};
    return [input.title, input.name].find(value => typeof value === 'string' && value.trim())?.trim() || '';
  });
  const targetReferences = computed(() => {
    const input = props.invocation.input ?? props.invocation.args ?? {};
    const references = [
      ['toolConfirmationCourse', input.course_id],
      ['toolConfirmationLesson', input.lesson_id],
    ].filter(([, id]) => isReference(id));
    if (!references.length && isReference(input.id)) {
      references.push(['toolConfirmationReference', input.id]);
    }
    return references.map(([key, id]) => t(key, { id }));
  });

  function isReference(value) {
    return Number.isSafeInteger(value) || (typeof value === 'string' && /^[\p{L}\p{N}_-]{1,64}$/u.test(value));
  }
</script>

<template>
  <section
    class="sveda-tool-approval grid gap-3 rounded-[max(0.625rem,var(--sveda-radius))] border border-border bg-card p-3"
    :aria-labelledby="titleId"
    :aria-busy="disabled"
  >
    <div class="flex items-start gap-2.5">
      <component
        :is="destructive ? TriangleAlert : ShieldCheck"
        class="mt-0.5 h-4 w-4 shrink-0"
        :class="destructive ? 'text-destructive' : 'text-muted-foreground'"
        aria-hidden="true"
      />
      <div class="min-w-0 space-y-1">
        <p :id="titleId" class="break-words text-sm font-semibold text-foreground [overflow-wrap:anywhere]">{{ action }}</p>
        <p v-if="targetName" class="break-words text-xs text-foreground [overflow-wrap:anywhere]">{{ targetName }}</p>
        <p v-if="targetReferences.length" class="break-words text-xs text-muted-foreground [overflow-wrap:anywhere]">
          {{ targetReferences.join(' · ') }}
        </p>
      </div>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <Button
        type="button"
        size="sm"
        class="rounded-[max(0.375rem,var(--sveda-radius))] text-xs"
        :disabled="disabled"
        @click="emit('confirm-tool', invocation.toolCallId, 'approve')"
      >
        {{ t(supportsScopedApproval ? 'toolConfirmOnce' : 'toolConfirm') }}
      </Button>
      <Button
        type="button"
        size="sm"
        variant="ghost"
        class="rounded-[max(0.375rem,var(--sveda-radius))] text-xs"
        :disabled="disabled"
        @click="emit('deny-tool', invocation.toolCallId)"
      >
        {{ t('toolDeny') }}
      </Button>
    </div>
    <div v-if="supportsScopedApproval" class="grid justify-items-start gap-1 border-t border-border pt-2">
      <Button
        type="button"
        size="sm"
        variant="ghost"
        class="max-w-full whitespace-normal rounded-[max(0.375rem,var(--sveda-radius))] px-2 text-left text-xs text-foreground"
        :disabled="disabled"
        @click="emit('confirm-tool', invocation.toolCallId, 'approve_for_chat')"
      >
        {{ t(scope === 'course' ? 'toolApproveForCourse' : 'toolApproveForChat') }}
      </Button>
      <p class="px-2 text-xs text-muted-foreground">
        {{ t(scope === 'course' ? 'toolCourseApprovalScope' : 'toolChatApprovalScope') }}
      </p>
    </div>
  </section>
</template>
