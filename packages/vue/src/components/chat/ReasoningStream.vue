<script setup>
  import { useAnimatedText } from '../../lib/useAnimatedText.js';
  import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

  const props = defineProps({
    animated: {
      type: Boolean,
      default: false,
    },
    text: {
      type: String,
      default: '',
    },
  });

  const emit = defineEmits(['content-resize']);

  const contentRef = ref(null);
  const translateY = ref(0);
  const hasOverflow = ref(false);
  let contentResizeObserver = null;

  const stripFencedCode = raw => {
    if (!raw || typeof raw !== 'string') return '';
    return raw
      .replace(/```[\w-]*\n?[\s\S]*?```/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  };

  const displayedText = useAnimatedText(() => stripFencedCode(props.text), {
    enabled: () => props.animated,
  });

  function updateTranslate() {
    if (!contentRef.value) return;
    const container = contentRef.value.parentElement;
    if (!container) return;
    const contentHeight = contentRef.value.offsetHeight;
    const containerHeight = container.offsetHeight;
    const overflow = Math.max(0, contentHeight - containerHeight);
    hasOverflow.value = overflow > 0;
    translateY.value = -overflow;
    emit('content-resize');
  }

  const scheduleTranslateUpdate = () => {
    nextTick(() => {
      requestAnimationFrame(updateTranslate);
    });
  };

  watch(displayedText, scheduleTranslateUpdate, { flush: 'post' });
  watch(() => props.text, scheduleTranslateUpdate, { flush: 'post' });

  onMounted(() => {
    scheduleTranslateUpdate();
    if (contentRef.value) {
      contentResizeObserver = new ResizeObserver(scheduleTranslateUpdate);
      contentResizeObserver.observe(contentRef.value);
    }
  });

  onUnmounted(() => {
    contentResizeObserver?.disconnect();
    contentResizeObserver = null;
  });
</script>

<template>
  <div
    class="reasoning-stream"
    :class="{ 'reasoning-stream--masked': hasOverflow }"
  >
    <div
      ref="contentRef"
      class="reasoning-stream__content"
      :class="{ 'reasoning-stream__content--following': animated }"
      :style="{ transform: `translateY(${translateY}px)` }"
    >
      {{ displayedText }}
    </div>
  </div>
</template>

<style scoped>
  .reasoning-stream {
    max-height: 10rem;
    overflow: hidden;
    position: relative;
  }

  .reasoning-stream--masked {
    mask-image: linear-gradient(to bottom, transparent 0%, black 12%, black 88%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 12%, black 88%, transparent 100%);
  }

  .reasoning-stream__content {
    font-family: ui-monospace, monospace;
    font-size: 0.7rem;
    line-height: 1.5;
    color: hsl(var(--muted-foreground) / 0.9);
    white-space: pre-wrap;
    word-break: break-word;
    padding: 0.25rem 0;
    transition: transform 0.3s ease-out;
  }

  .reasoning-stream__content--following {
    transition: none;
  }
</style>
