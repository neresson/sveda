<script setup>
  import { Button } from '../../ui/button';
  import { ArrowDown } from 'lucide-vue-next';

  defineProps({
    label: { type: String, required: true },
  });

  const emit = defineEmits(['scroll']);
</script>

<template>
  <div class="flex shrink-0 justify-center px-4 py-2">
    <Button
      type="button"
      variant="secondary"
      size="sm"
      class="h-8 gap-2 rounded-full border border-border bg-background/95 px-3 text-xs shadow-sm backdrop-blur"
      @click="emit('scroll')"
    >
      <span>{{ label }}</span>
      <ArrowDown class="h-4 w-4" />
    </Button>
  </div>
</template>
