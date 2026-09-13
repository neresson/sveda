<script setup>
  import { useAnimatedText } from '../../lib/useAnimatedText.js';
  import { nextTick, ref, watch } from 'vue';

  const props = defineProps({
    animated: {
      type: Boolean,
      default: false,
    },
    text: {
      type: String,
      default: '',
    },
    maxHeight: {
      type: String,
      default: '10rem',
    },
    showMask: {
      type: Boolean,
      default: true,
    },
    variant: {
      type: String,
      default: 'mono',
    },
  });

  const contentRef = ref(null);
  const translateY = ref(0);
  const displayedText = useAnimatedText(() => props.text, {
    enabled: () => props.animated,
  });

  function updateTranslate() {
    if (!contentRef.value) return;
    const container = contentRef.value.parentElement;
    if (!container) return;
    const contentHeight = contentRef.value.offsetHeight;
    const containerHeight = container.offsetHeight;
    const overflow = Math.max(0, contentHeight - containerHeight);
    translateY.value = -overflow;
  }

  watch(
    displayedText,
    () => {
      nextTick(updateTranslate);
    },
    { flush: 'post' }
  );
</script>

<template>
  <div
    class="animated-stream-block"
    :class="{ 'animated-stream-block--masked': showMask }"
    :style="{ height: maxHeight }"
  >
    <div
      ref="contentRef"
      class="animated-stream-block__content"
      :class="{ 'animated-stream-block__content--mono': variant === 'mono' }"
      :style="{ transform: `translateY(${translateY}px)` }"
    >
      {{ displayedText }}
    </div>
  </div>
</template>

<style scoped>
  .animated-stream-block {
    overflow: hidden;
    position: relative;
  }

  .animated-stream-block--masked {
    mask-image: linear-gradient(to bottom, transparent 0%, black 12%, black 88%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 12%, black 88%, transparent 100%);
  }

  .animated-stream-block__content {
    font-size: 0.7rem;
    line-height: 1.5;
    color: hsl(var(--muted-foreground) / 0.9);
    white-space: pre-wrap;
    word-break: break-word;
    padding: 0.25rem 0;
    transition: transform 0.3s ease-out;
  }

  .animated-stream-block__content--mono {
    font-family: ui-monospace, monospace;
  }
</style>
