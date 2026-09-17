<script setup>
  defineProps({
    isMobile: { type: Boolean, required: true },
    viewMode: { type: String, required: true },
  });

  defineEmits(['start-resize']);
</script>

<template>
  <div
    v-if="!isMobile && viewMode === 'fixed'"
    class="absolute bottom-3 left-0 top-3 z-10 w-1 cursor-ew-resize hover:bg-primary/30"
    @mousedown="$emit('start-resize', 'fixed-left', $event)"
  ></div>
  <div
    v-if="!isMobile && viewMode === 'floating'"
    class="absolute left-0 top-0 z-10 h-3 w-3 cursor-nw-resize rounded-br hover:bg-primary/20"
    @mousedown="$emit('start-resize', 'top-left', $event)"
  ></div>
  <div
    v-if="!isMobile && viewMode === 'floating'"
    class="absolute left-3 right-3 top-0 z-10 h-1 cursor-n-resize hover:bg-primary/30"
    @mousedown="$emit('start-resize', 'top', $event)"
  ></div>
  <div
    v-if="!isMobile && viewMode === 'floating'"
    class="absolute bottom-3 left-0 top-3 z-10 w-1 cursor-w-resize hover:bg-primary/30"
    @mousedown="$emit('start-resize', 'left', $event)"
  ></div>
</template>
