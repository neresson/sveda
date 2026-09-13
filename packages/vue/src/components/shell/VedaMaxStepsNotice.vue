<script setup>
  import { Button } from '../../ui/button';
  import { Play } from 'lucide-vue-next';

  defineProps({
    message: { type: String, required: true },
    continueLabel: { type: String, required: true },
    loading: { type: Boolean, default: false },
  });

  const emit = defineEmits(['continue']);
</script>

<template>
  <div class="flex shrink-0 justify-center px-4 py-2">
    <div class="flex max-w-md flex-col items-center gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-center shadow-sm">
      <p class="text-xs text-foreground">
        {{ message }}
      </p>
      <Button
        type="button"
        variant="secondary"
        size="sm"
        class="h-8 gap-2 rounded-full px-3 text-xs"
        :disabled="loading"
        @click="emit('continue')"
      >
        <Play class="h-3.5 w-3.5" />
        {{ continueLabel }}
      </Button>
    </div>
  </div>
</template>
