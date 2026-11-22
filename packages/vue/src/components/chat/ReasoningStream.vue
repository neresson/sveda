<script setup>
  import { computed } from 'vue';

  const props = defineProps({
    text: { type: String, default: '' },
  });

  const preview = computed(() => props.text
    .replace(/```[^\n]*\n[\s\S]*?(?:```|$)/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 600)
  );
</script>

<template>
  <p v-if="preview" class="reasoning-stream">
    {{ preview }}
  </p>
</template>

<style scoped>
  .reasoning-stream {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    max-height: 3lh;
    overflow: clip;
    overflow-wrap: anywhere;
    margin: 0;
    color: var(--color-muted-foreground);
    font-size: 0.75rem;
    line-height: 1.5;
  }
</style>
