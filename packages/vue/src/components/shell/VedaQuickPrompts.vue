<script setup>
  import { Button } from '../../ui/button';

  defineProps({
    prompts: { type: Array, required: true },
    disabled: { type: Boolean, default: false },
    title: { type: String, required: true },
  });

  defineEmits(['select']);
</script>

<template>
  <div
    class="w-full"
    role="group"
    :aria-label="title"
  >
    <div class="quick-prompts-row -mx-1 flex gap-2 overflow-x-auto px-1 pb-1">
      <Button
        v-for="prompt in prompts"
        :key="prompt.label"
        type="button"
        variant="secondary"
        size="sm"
        class="h-8 shrink-0 rounded-[var(--veda-radius)] border border-border/60 bg-muted/80 px-3 text-xs font-normal text-muted-foreground shadow-none hover:bg-muted hover:text-foreground"
        :disabled="disabled"
        @click="$emit('select', prompt.prompt)"
      >
        {{ prompt.label }}
      </Button>
    </div>
  </div>
</template>

<style scoped>
  .quick-prompts-row {
    scrollbar-color: hsl(var(--foreground) / 0.2) transparent;
  }

  .quick-prompts-row::-webkit-scrollbar-track {
    background: transparent;
  }
</style>
